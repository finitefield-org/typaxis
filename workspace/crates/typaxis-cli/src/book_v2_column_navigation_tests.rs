use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_navigation as navigation_drive;
use typaxis_display_list::book_v2::BookV2NavigationDisplay;
use typaxis_pdf::book_v2::{
    BookV2ColumnNavigationGeometry, BookV2ColumnNavigationGeometryBuilder as Navigation,
    BookV2DestinationKind as DestinationKind, BookV2NavigationGeometryError as NavigationError,
    BookV2NavigationTarget as Target,
};
use typaxis_syntax::{
    ProductionInlineLinkTarget as LinkTarget, ProductionInlineReference as Reference,
};

#[path = "book_v2_column_navigation_continuation_tests.rs"]
mod continuation_tests;
#[path = "book_v2_column_navigation_probes.rs"]
mod navigation_probes;

#[path = "book_v2_column_region_tests.rs"]
mod region_tests;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedRectangle {
    link: usize,
    page: u32,
    fragment: usize,
    column: Option<u16>,
    group: usize,
    bounds: [i64; 4],
    next: Option<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum ExpectedTarget {
    Destination(usize),
    Uri(String),
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct ExpectedLink {
    node: usize,
    target: ExpectedTarget,
    rectangles: Vec<usize>,
}
#[derive(Clone, Debug)]
struct ExpectedNavigation {
    destinations: Vec<(u32, Value, Option<[i64; 4]>)>,
    links: Vec<ExpectedLink>,
    rectangles: Vec<ExpectedRectangle>,
    pages: Vec<std::ops::Range<usize>>,
    outline: Vec<(usize, Option<usize>, Vec<usize>)>,
}
fn raw_rect(r: typaxis_core::Rect) -> [i64; 4] {
    [
        r.x().raw(),
        r.y().raw(),
        r.width().get().raw(),
        r.height().get().raw(),
    ]
}
fn union_rect(a: [i64; 4], b: [i64; 4]) -> [i64; 4] {
    let x = a[0].min(b[0]);
    let y = a[1].min(b[1]);
    [
        x,
        y,
        (a[0] + a[2]).max(b[0] + b[2]) - x,
        (a[1] + a[3]).max(b[1] + b[3]) - y,
    ]
}
fn component_bounds(
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    paint: usize,
) -> Option<typaxis_core::Rect> {
    match display.paints()[paint] {
        Paint::PageRegionText { .. } => panic!("running region outside the legacy Column fixture"),
        Paint::Text(i) => display.text().draws()[i].logical_bounds(),
        Paint::Marker(i) => Some(display.markers().draws()[i].placement().bounds()),
        Paint::Math(i) => Some(match display.math().draws()[i].paint() {
            BookV2MathPaint::Native(n) => n.bounds(),
            BookV2MathPaint::Vector(v) => v.viewport(),
        }),
        Paint::Image(i) => Some(display.images().draws()[i].paint().viewport()),
        Paint::EquationNumber(i) => {
            Some(display.numbers().draws()[i].placement().geometry().bounds())
        }
        Paint::FootnoteSeparator(_) => {
            panic!("artifact separator has no semantic navigation bounds")
        }
    }
}

// Original source parents and actual component bounds supply the expectation.
// The navigation bounds/union helpers are never used to construct it.
fn expected_navigation(
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
) -> Result<ExpectedNavigation, NavigationError> {
    inspect_relations(tree);
    let registry = tree.source();
    let marked = registry.source();
    let scopes = marked.source();
    let display = scopes.display();
    let original = display
        .source()
        .original_graph()
        .lines()
        .prepared()
        .source_flow();
    let wire = package_wire(display);
    let mut authored_links = BTreeMap::new();
    let mut authored_references = BTreeMap::new();
    fn scan(v: &Value, links: &mut BTreeMap<u32, Value>, references: &mut BTreeMap<u32, Value>) {
        match v {
            Value::Object(o) => {
                if v["kind"] == "link" {
                    assert!(links
                        .insert(v["node_id"].as_u64().unwrap() as u32, v["target"].clone())
                        .is_none());
                }
                if v["kind"] == "reference" {
                    assert!(references
                        .insert(v["node_id"].as_u64().unwrap() as u32, v.clone())
                        .is_none());
                }
                for v in o.values() {
                    scan(v, links, references);
                }
            }
            Value::Array(a) => {
                for v in a {
                    scan(v, links, references);
                }
            }
            _ => (),
        }
    }
    scan(
        &wire["document"],
        &mut authored_links,
        &mut authored_references,
    );
    let lookup = registry
        .nodes()
        .iter()
        .enumerate()
        .map(|(i, n)| ((n.source().key().owner().get(), n.source().key().slot()), i))
        .collect::<BTreeMap<_, _>>();
    let locate = |owner, slot| lookup[&(owner, slot)];
    let mut destinations = original
        .navigation()
        .anchors()
        .iter()
        .map(|(id, owner)| (owner.get(), json!(["anchor", id.as_str()]), None))
        .collect::<Vec<_>>();
    let na = destinations.len();
    for (i, n) in tree.notes().iter().enumerate() {
        destinations.push((
            registry.nodes()[n.node_index()]
                .source()
                .key()
                .owner()
                .get(),
            json!(["footnote", i]),
            None,
        ));
    }
    let nd = tree.notes().len();
    for (i, e) in tree.edges().iter().enumerate() {
        destinations.push((
            registry.nodes()[e.reference_node()]
                .source()
                .key()
                .owner()
                .get(),
            json!(["reference", i]),
            None,
        ));
    }
    let anchor = |name: &str| {
        original
            .navigation()
            .anchors()
            .iter()
            .position(|(id, _)| id.as_str() == name)
            .unwrap()
    };
    let mut links = Vec::<ExpectedLink>::new();
    for p in original.paragraphs() {
        for site in p.items() {
            let expected = match (site.link_target(), site.reference()) {
                (Some(LinkTarget::Internal { anchor_id }), None) => {
                    assert_eq!(
                        authored_links[&site.owner().get()],
                        json!({"kind":"internal","anchor_id":anchor_id})
                    );
                    Some((Slot::Source, ExpectedTarget::Destination(anchor(anchor_id))))
                }
                (Some(LinkTarget::Uri { uri }), None) => {
                    assert_eq!(
                        authored_links[&site.owner().get()],
                        json!({"kind":"uri","uri":uri})
                    );
                    Some((Slot::Source, ExpectedTarget::Uri(uri.to_owned())))
                }
                (None, Some(Reference::Anchor { target, .. })) => {
                    assert_eq!(authored_references[&site.owner().get()]["target"], target);
                    Some((
                        Slot::ReferenceLink,
                        ExpectedTarget::Destination(anchor(target)),
                    ))
                }
                (None, _) => None,
                _ => panic!("ambiguous original link site"),
            };
            if let Some((slot, target)) = expected {
                links.push(ExpectedLink {
                    node: locate(site.owner().get(), slot),
                    target,
                    rectangles: Vec::new(),
                });
            }
        }
    }
    for edge in tree.edges() {
        links.push(ExpectedLink {
            node: edge.reference_link(),
            target: ExpectedTarget::Destination(na + edge.definition_index()),
            rectangles: Vec::new(),
        });
    }
    for note in tree.notes() {
        if let Some(edge) = note.return_reference() {
            links.push(ExpectedLink {
                node: note.link_node(),
                target: ExpectedTarget::Destination(na + nd + edge),
                rectangles: Vec::new(),
            });
        }
    }
    let roots = links
        .iter()
        .enumerate()
        .map(|(i, l)| (l.node, i))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(roots.len(), links.len());
    let parents = registry
        .nodes()
        .iter()
        .map(|n| n.parent())
        .collect::<Vec<_>>();
    let mut active = vec![None; parents.len()];
    for i in 0..parents.len() {
        active[i] = roots.get(&i).copied();
        if let Some(p) = parents[i] {
            assert!(p < i);
            if active[i].is_some() && active[p].is_some() {
                return Err(NavigationError::ConflictingLinks(
                    registry.nodes()[i].source().key().owner(),
                ));
            }
            if active[i].is_none() {
                active[i] = active[p];
            }
        }
    }
    let mut first = vec![None::<(u32, usize, [i64; 4])>; parents.len()];
    for empty in display.anchors().nonpainting_lines() {
        if empty.repeated_header() {
            continue;
        }
        let f = empty.fragment().fragment();
        let value = (f.page_index(), empty.fragment_index(), raw_rect(f.bounds()));
        let mut at = Some(locate(f.owner().get(), Slot::Source));
        while let Some(i) = at {
            if first[i].is_none() {
                first[i] = Some(value);
            }
            at = parents[i];
        }
    }
    let mut rectangles = Vec::<ExpectedRectangle>::new();
    let mut pages = Vec::new();
    for (page, p) in marked.pages().iter().enumerate() {
        let start = rectangles.len();
        for group in p.groups() {
            let g = &scopes.groups()[group];
            let Some(binding) = registry.for_group(group) else {
                assert!(g.artifact().is_some());
                continue;
            };
            assert!(g.artifact().is_none());
            let mut bounds = None;
            for pi in g.paints() {
                if let Some(r) = component_bounds(display, pi) {
                    bounds =
                        Some(bounds.map_or(raw_rect(r), |prior| union_rect(prior, raw_rect(r))));
                }
            }
            let Some(bounds) = bounds else {
                continue;
            };
            let value = (page as u32, g.fragment_index(), bounds);
            let mut at = Some(binding.node_index());
            while let Some(i) = at {
                first[i] = match first[i] {
                    None => Some(value),
                    Some(old) if (value.0, value.1) < (old.0, old.1) => Some(value),
                    Some(old) if (value.0, value.1) == (old.0, old.1) => {
                        Some((old.0, old.1, union_rect(old.2, bounds)))
                    }
                    other => other,
                };
                at = parents[i];
            }
            if let Some(li) = active[binding.node_index()] {
                if let Some(&last) = links[li].rectangles.last() {
                    if (rectangles[last].page, rectangles[last].fragment) == (value.0, value.1) {
                        rectangles[last].bounds = union_rect(rectangles[last].bounds, bounds);
                        continue;
                    }
                }
                let index = rectangles.len();
                if let Some(&last) = links[li].rectangles.last() {
                    rectangles[last].next = Some(index);
                }
                links[li].rectangles.push(index);
                rectangles.push(ExpectedRectangle {
                    link: li,
                    page: value.0,
                    fragment: value.1,
                    column: display.fragment_column(value.1),
                    group,
                    bounds,
                    next: None,
                });
            }
        }
        pages.push(start..rectangles.len());
    }
    for d in &mut destinations[..na] {
        if let Some(&i) = lookup.get(&(d.0, Slot::Source)) {
            d.2 = first[i].map(|(p, f, b)| [i64::from(p), f as i64, b[0], b[1]]);
        }
    }
    for a in display.anchors().positions() {
        if a.repeated_header() {
            continue;
        }
        let d = destinations[..na]
            .iter_mut()
            .find(|d| d.0 == a.anchor().source().owner().get())
            .unwrap();
        assert!(d.2.is_none());
        d.2 = Some([
            i64::from(a.fragment().fragment().page_index()),
            a.fragment_index() as i64,
            a.x().raw(),
            a.baseline().raw(),
        ]);
    }
    for (i, n) in tree.notes().iter().enumerate() {
        destinations[na + i].2 =
            first[n.node_index()].map(|(p, f, b)| [i64::from(p), f as i64, b[0], b[1]]);
        assert_eq!(destinations[na + i].2.is_some(), n.painted());
    }
    for (i, e) in tree.edges().iter().enumerate() {
        destinations[na + nd + i].2 =
            first[e.reference_label()].map(|(p, f, b)| [i64::from(p), f as i64, b[0], b[1]]);
        assert_eq!(destinations[na + nd + i].2.is_some(), e.painted());
    }
    for l in &links {
        if !l.rectangles.is_empty() {
            if let ExpectedTarget::Destination(i) = l.target {
                if destinations[i].2.is_none() {
                    return Err(NavigationError::UnplacedDestination(
                        typaxis_core::NodeId::new(destinations[i].0),
                    ));
                }
            }
        }
    }
    let mut outline = Vec::<(usize, Option<usize>, Vec<usize>)>::new();
    for (i, e) in original.navigation().outline().iter().enumerate() {
        let destination = anchor(e.destination.as_str());
        if destinations[destination].2.is_none() {
            return Err(NavigationError::UnplacedDestination(
                typaxis_core::NodeId::new(destinations[destination].0),
            ));
        }
        let parent = e.parent_outline_id.map(|id| {
            original.navigation().outline()[..i]
                .iter()
                .position(|e| e.outline_id == id)
                .unwrap()
        });
        outline.push((destination, parent, Vec::new()));
        if let Some(p) = parent {
            outline[p].2.push(i);
        }
    }
    Ok(ExpectedNavigation {
        destinations,
        links,
        rectangles,
        pages,
        outline,
    })
}

fn inspect_navigation(
    nav: &BookV2ColumnNavigationGeometry<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
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
) -> ExpectedNavigation {
    let expected = expected_navigation(nav.source()).unwrap();
    let registry = nav.source().source();
    let display = registry.source().source().display();
    assert_eq!(nav.output_charge(), nav.source().output_charge());
    assert_eq!(nav.destinations().len(), expected.destinations.len());
    for (d, (owner, kind, position)) in nav.destinations().iter().zip(&expected.destinations) {
        assert_eq!(d.owner().get(), *owner);
        let actual = match d.kind() {
            DestinationKind::Anchor(s) => json!(["anchor", s]),
            DestinationKind::Footnote(i) => json!(["footnote", i]),
            DestinationKind::Reference(i) => json!(["reference", i]),
        };
        assert_eq!(actual, *kind);
        assert_eq!(
            d.position().map(|p| [
                i64::from(p.page_index()),
                p.fragment_index() as i64,
                p.x().raw(),
                p.y().raw()
            ]),
            *position
        );
    }
    assert_eq!(nav.links().len(), expected.links.len());
    for (l, e) in nav.links().iter().zip(&expected.links) {
        assert_eq!(l.structure_node(), e.node);
        assert_eq!(l.first_rectangle(), e.rectangles.first().copied());
        assert_eq!(
            match l.target() {
                Target::Destination(i) => ExpectedTarget::Destination(i),
                Target::Uri(s) => ExpectedTarget::Uri(s.into()),
            },
            e.target
        );
    }
    assert_eq!(nav.rectangles().len(), expected.rectangles.len());
    for (r, e) in nav.rectangles().iter().zip(&expected.rectangles) {
        assert_eq!(
            (
                r.link_index(),
                r.page_index(),
                r.fragment_index(),
                r.first_group(),
                raw_rect(r.bounds()),
                r.next_for_link()
            ),
            (e.link, e.page, e.fragment, e.group, e.bounds, e.next)
        );
        assert!(registry.for_group(r.first_group()).is_some());
    }
    for (p, range) in expected.pages.iter().enumerate() {
        assert_eq!(
            nav.page_rectangles(p).unwrap(),
            &nav.rectangles()[range.clone()]
        );
    }
    assert!(nav.page_rectangles(expected.pages.len()).is_none());
    assert_eq!(nav.outline().len(), expected.outline.len());
    for (i, (o, (destination, parent, children))) in
        nav.outline().iter().zip(&expected.outline).enumerate()
    {
        let siblings = parent.map_or_else(
            || {
                expected
                    .outline
                    .iter()
                    .enumerate()
                    .filter(|(_, o)| o.1.is_none())
                    .map(|(j, _)| j)
                    .collect::<Vec<_>>()
            },
            |p| expected.outline[p].2.clone(),
        );
        let si = siblings.iter().position(|&j| j == i).unwrap();
        let descendants = expected
            .outline
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .filter(|(j, _)| {
                let mut at = expected.outline[*j].1;
                while let Some(p) = at {
                    if p == i {
                        return true;
                    }
                    at = expected.outline[p].1;
                }
                false
            })
            .count();
        assert_eq!(
            (
                o.destination(),
                o.parent(),
                o.first_child(),
                o.last_child(),
                o.previous(),
                o.next(),
                o.descendant_count()
            ),
            (
                *destination,
                *parent,
                children.first().copied(),
                children.last().copied(),
                si.checked_sub(1).map(|j| siblings[j]),
                siblings.get(si + 1).copied(),
                descendants
            )
        );
    }
    assert!(display.navigation_paint_bounds(usize::MAX).is_err());
    assert!(display
        .navigation_anchor(display.navigation_anchor_count())
        .is_none());
    assert!(display
        .navigation_nonpainting_line(display.navigation_nonpainting_line_count())
        .is_none());
    expected
}

fn input_navigation(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> ExpectedNavigation {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    navigation_drive(
        input,
        caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            assert!(std::ptr::eq(
                bundle.navigation().source(),
                bundle.relations()
            ));
            let expected = inspect_navigation(bundle.navigation());
            assert_eq!(
                observed.output_charge(),
                bundle.relations_prefix().output_charge()
            );
            assert_eq!(
                bundle.constructor_charges()[12][2],
                bundle.relations().output_charge()
            );
            let n = bundle.navigation();
            navigation_probes::emit(
                case,
                Some(n),
                bundle.relations(),
                None,
                caps,
                navigation_probes::Context {
                    scope: "navigation_driver",
                    parent_scope: "relation_driver",
                    maximum: MAXIMUM,
                    constructors: bundle.constructor_charges(),
                    font_units: bundle.marked().paints().reservation_sizes(),
                    first_bounds_size: builder.reservation_sizes()[0],
                    encoded: Some(bundle.encoded_prefix()),
                    marked: Some(bundle.marked_prefix()),
                    relations: Some(bundle.relations_prefix()),
                    ancestor: Some(bundle.relations_prefix()),
                    parent_caller_before: Some(bundle.caller_before()),
                    caller_before: Some(bundle.caller_before()),
                    observed: Some(observed),
                    build_after: [
                        n.record_charge(),
                        n.spool_charge(),
                        n.output_charge(),
                        n.work_steps(),
                    ],
                },
            );
            expected
        },
    )
    .unwrap_or_else(|e| panic!("{case}: {e:?}"))
}

#[test]
fn book_v2_column_navigation_same_original_link_crosses_two_three_columns_and_pages() {
    let caps = display_caps();
    for columns in [2, 3] {
        for balanced in [false, true] {
            let mut data = description_data();
            if columns == 3 {
                for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
                    m["width"] = 14_000_000.into();
                    m["trim"]["width"] = 14_000_000.into();
                    m["body"]["width"] = 12_200_001.into();
                    m["column_layout"]["count"] = 3.into();
                }
            }
            balance(&mut data, balanced);
            let root = Root::new();
            let input = display_input(&root, data, None, &caps);
            let facts = input_navigation(
                &format!("navigation-description-{columns}-{balanced}"),
                &input,
                &caps,
            );
            assert_eq!(facts.links.len(), 1);
            let pages = facts
                .rectangles
                .iter()
                .map(|r| r.page)
                .collect::<BTreeSet<_>>();
            assert!(pages.len() > 1);
            assert_eq!(
                facts
                    .rectangles
                    .iter()
                    .filter(|r| r.page == 0)
                    .map(|r| r.column.unwrap())
                    .collect::<BTreeSet<_>>(),
                (0..columns as u16).collect::<BTreeSet<_>>()
            );
            // Every selected original term line is a distinct global fragment.
            assert_eq!(facts.links[0].rectangles.len(), 12);
            assert_eq!(
                facts
                    .rectangles
                    .iter()
                    .map(|r| (r.page, r.fragment))
                    .collect::<BTreeSet<_>>()
                    .len(),
                12
            );
        }
    }
}

fn run_document(
    case: &str,
    data: Value,
    kind: &str,
    caps: &M4EffectiveResourceLimits,
) -> ExpectedNavigation {
    let root = Root::new();
    let input = match kind {
        "native" => {
            let text = data["text_buffers"][0]["utf8"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec();
            native_input_with_source(&root, data, &text, caps)
        }
        "vector" => vector_input(&root, data, caps),
        "png" => figure_input(&root, data, PNG, caps),
        "jpeg" => figure_input(&root, data, JPEG, caps),
        "svg" => figure_input(&root, data, SVG, caps),
        "list" => display_input(&root, data, Some(DISPLAY_LIST_FONT), caps),
        _ => display_input(&root, data, None, caps),
    };
    input_navigation(case, &input, caps)
}

#[test]
fn book_v2_column_navigation_original_media_math_headers_and_table_sources_remain_physical() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for balanced in [false, true] {
            let mut data=super::super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
            anchors(&mut data);
            balance(&mut data, balanced);
            let facts = run_document(
                &format!("navigation-original-media-{kind}-{balanced}"),
                data,
                kind,
                &caps,
            );
            assert!(facts.destinations.iter().any(|d| d.2.is_some()));
        }
    }
    for vector in [false, true] {
        for notes in [false, true] {
            let mut data = header_data(vector, notes);
            let expected_notes = data["document"]["footnotes"].as_array().unwrap().len();
            anchors(&mut data);
            balance(&mut data, !notes);
            let facts = run_document(
                &format!("navigation-original-header-{vector}-{notes}"),
                data,
                if vector { "vector" } else { "native" },
                &caps,
            );
            assert!(facts.pages.len() > 1);
            assert_eq!(
                facts
                    .destinations
                    .iter()
                    .filter(|d| d.1[0] == "footnote")
                    .count(),
                expected_notes
            );
        }
    }
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
        let facts = run_document(
            &format!("navigation-original-table-{mode}"),
            data,
            "body",
            &caps,
        );
        assert!(facts.pages.len() > 0);
    }
    for repeated in [false, true] {
        let facts = run_document(
            &format!("navigation-original-fraction-{repeated}"),
            fraction_data(repeated),
            "native",
            &caps,
        );
        assert!(facts.destinations.len() > 0);
    }
}

#[test]
fn book_v2_column_navigation_note_reading_and_return_targets_use_original_source_ancestry() {
    let caps = display_caps();
    for cycle in [false, true] {
        let root = Root::new();
        let input = display_input(
            &root,
            dependency_data(cycle),
            Some(DISPLAY_LIST_FONT),
            &caps,
        );
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, builder, observed| {
                let facts = inspect_navigation(bundle.navigation());
                let tree = bundle.relations();
                let na = bundle.display().navigation_source().anchors().len();
                for note in tree.notes() {
                    if let Some(returning) = note.return_reference() {
                        let link = facts
                            .links
                            .iter()
                            .find(|l| l.node == note.link_node())
                            .unwrap();
                        assert_eq!(
                            link.target,
                            ExpectedTarget::Destination(na + tree.notes().len() + returning)
                        );
                        for &r in &link.rectangles {
                            assert_eq!(
                                bundle
                                    .display()
                                    .fragment_column(facts.rectangles[r].fragment),
                                None
                            );
                        }
                    }
                }
                assert!(tree.notes().iter().any(|n| n.painted()));
                if !cycle {
                    assert!(tree
                        .notes()
                        .iter()
                        .any(|n| n.reading_reference() != n.return_reference()));
                }
                let n = bundle.navigation();
                navigation_probes::emit(
                    &format!("navigation-original-note-cycle-{cycle}"),
                    Some(n),
                    tree,
                    None,
                    &caps,
                    navigation_probes::Context {
                        scope: "navigation_driver",
                        parent_scope: "relation_driver",
                        maximum: MAXIMUM,
                        constructors: bundle.constructor_charges(),
                        font_units: bundle.marked().paints().reservation_sizes(),
                        first_bounds_size: builder.reservation_sizes()[0],
                        encoded: Some(bundle.encoded_prefix()),
                        marked: Some(bundle.marked_prefix()),
                        relations: Some(bundle.relations_prefix()),
                        ancestor: Some(bundle.relations_prefix()),
                        parent_caller_before: Some(bundle.caller_before()),
                        caller_before: Some(bundle.caller_before()),
                        observed: Some(observed),
                        build_after: [
                            n.record_charge(),
                            n.spool_charge(),
                            n.output_charge(),
                            n.work_steps(),
                        ],
                    },
                );
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_navigation_uri_outline_and_exact_text_number_targets_keep_source_metadata() {
    let caps = display_caps();
    let mut data = description_data();
    let uri = format!(
        "https://example.invalid/read?x=(a)&label={}日本語",
        "z".repeat(100)
    );
    data["document"]["blocks"][0]["items"][0]["term"]["children"][0]["target"] =
        json!({"kind":"uri","uri":uri});
    let facts = run_document("navigation-uri-utf8-chunks", data, "body", &caps);
    assert_eq!(facts.links[0].target, ExpectedTarget::Uri(uri));
    let mut data = description_data();
    let mut heading = data["document"]["blocks"][0]["items"][0]["blocks"][0].clone();
    heading["kind"] = "heading".into();
    heading["level"] = 2.into();
    heading["anchor_id"] = "chapter.second".into();
    heading["children"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v["kind"] != "anchor");
    let mut first = heading.clone();
    first["anchor_id"] = "chapter.first".into();
    first["level"] = 1.into();
    let mut rule = data["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["style_id"] == "paragraph-text")
        .unwrap()
        .clone();
    rule["style_id"] = "navigation-heading".into();
    rule["selector"] = "heading".into();
    rule["source_order"] = data["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .len()
        .into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    let blocks = data["document"]["blocks"].as_array_mut().unwrap();
    blocks.insert(0, first);
    blocks.push(heading);
    semantic_renumber(&mut data["document"], &mut 0);
    let first_owner = data["document"]["blocks"][0]["node_id"].clone();
    let last_owner = data["document"]["blocks"][2]["node_id"].clone();
    data["outline"]["entries"] = json!([
        {"outline_id":0,"parent_outline_id":null,"level":1,"destination":"chapter.first","label":"Book 本","source_node_id":first_owner,"source_kind":"heading"},
        {"outline_id":1,"parent_outline_id":0,"level":2,"destination":"chapter.second","label":"章 — second","source_node_id":last_owner,"source_kind":"heading"}
    ]);
    let facts = run_document("navigation-outline-authored-source", data, "body", &caps);
    assert_eq!(facts.outline.len(), 2);
    let facts = run_document(
        "navigation-original-number-references",
        number_reference_data(),
        "body",
        &caps,
    );
    assert_eq!(facts.links.len(), 2);
}

fn semantic_refusal_data(conflict: bool) -> Value {
    let mut data = if conflict {
        description_data()
    } else {
        plain_data("Pro", false, false)
    };
    let plain = plain_data("Pro", true, false);
    let mut note = plain["document"]["footnotes"][0].clone();
    note["footnote_id"] = "relation-term-note".into();
    if conflict {
        let link = &mut data["document"]["blocks"][0]["items"][0]["term"]["children"][0];
        let span = link["span"].clone();
        link["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"relation-term-note"}));
    } else {
        let p = &mut note["blocks"][0];
        let span = p["span"].clone();
        p["children"].as_array_mut().unwrap().push(
            json!({"kind":"anchor","node_id":0,"span":span,"anchor_id":"unused.navigation.target"}),
        );
        let p = &mut data["document"]["blocks"][0];
        p["children"] = json!([{"kind":"link","node_id":0,"span":p["span"],"target":{"kind":"internal","anchor_id":"unused.navigation.target"},"children":p["children"]}]);
    }
    data["document"]["footnotes"] = json!([note]);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        let body = m["body"].clone();
        let bottom = m["height"].as_i64().unwrap();
        m["footnote"] =
            json!({"x":body["x"],"y":bottom-3_000_000,"width":body["width"],"height":2_000_000});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    data
}

#[test]
fn book_v2_column_navigation_semantic_refusals_keep_original_owners_and_spent_prefixes() {
    let caps = display_caps();
    for variant in 0..4 {
        let conflict = variant == 0 || variant == 2;
        let mut data = semantic_refusal_data(conflict);
        if variant == 2 {
            data["document"]["blocks"][0]["items"][0]["term"]["children"][0]["target"] =
                json!({"kind":"uri","uri":"https://example.invalid/conflicting-original-note"});
        }
        if variant == 3 {
            let children = data["document"]["blocks"][0]["children"][0]["children"].clone();
            data["document"]["blocks"][0]["children"] = children;
            let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
            data["document"]["footnotes"][0]["blocks"] = json!([{"kind":"semantic_container","node_id":0,"classes":[],"semantic_kind":"solution","anchor_id":"unused.outline.target","span":paragraph["span"],"blocks":[paragraph]}]);
            semantic_renumber(&mut data["document"], &mut 0);
            let owner = data["document"]["footnotes"][0]["blocks"][0]["node_id"].clone();
            data["outline"]["entries"] = json!([{"outline_id":0,"parent_outline_id":null,"level":1,"destination":"unused.outline.target","label":"Unplaced 本","source_node_id":owner,"source_kind":"semantic_container"}]);
        }
        let root = Root::new();
        let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut delivered = false;
        let error = navigation_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
            delivered = true;
        })
        .unwrap_err();
        assert!(!delivered);
        assert!(matches!(
            error,
            crate::book_v2_resources::BookV2ConvergenceError::Stage {
                stage: "column navigation geometry",
                ..
            }
        ));
        let driver_failure = paint_observation(budget.observation());
        assert!(
            navigation_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
                delivered = true;
            })
            .is_err()
        );
        assert!(!delivered);
        let retry = paint_observation(budget.observation());
        for axis in [0, 1, 3] {
            assert!(retry[axis] > driver_failure[axis]);
        }
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        relation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let expected = expected_navigation(bundle.relations()).unwrap_err();
                assert_eq!(
                    matches!(expected, NavigationError::ConflictingLinks(_)),
                    conflict
                );
                let mut r = observed.record_charge();
                let mut s = observed.spool_charge();
                let mut o = observed.output_charge();
                let mut w = observed.work_steps();
                let mut builder = Navigation::new_counted(
                    bundle.relations(),
                    &caps,
                    MAXIMUM,
                    r,
                    s,
                    o,
                    w,
                    &mut r,
                    &mut s,
                    &mut o,
                    &mut w,
                )
                .unwrap();
                let mut constructors = [[0; 4]; 13];
                constructors[..12].copy_from_slice(&bundle.constructor_charges());
                constructors[12] = [r, s, o, w];
                let first = builder.build().err().unwrap();
                assert_eq!(first, expected);
                let after = [
                    builder.record_charge(),
                    builder.spool_charge(),
                    builder.output_charge(),
                    builder.work_steps(),
                ];
                assert_eq!(after, driver_failure);
                assert!(
                    after[0] > constructors[12][0]
                        && after[1] > constructors[12][1]
                        && after[3] > constructors[12][3]
                );
                assert_eq!(after[2], observed.output_charge());
                navigation_probes::emit(
                    &format!("navigation-semantic-refusal-{variant}-{conflict}"),
                    None,
                    bundle.relations(),
                    Some(&first),
                    &caps,
                    navigation_probes::Context {
                        scope: "relation_callback",
                        parent_scope: "relation_driver",
                        maximum: MAXIMUM,
                        constructors,
                        font_units: bundle.marked().paints().reservation_sizes(),
                        first_bounds_size: builder.reservation_sizes()[0],
                        encoded: Some(bundle.encoded_prefix()),
                        marked: Some(bundle.marked_prefix()),
                        relations: Some(observed),
                        ancestor: Some(observed),
                        parent_caller_before: Some(bundle.caller_before()),
                        caller_before: None,
                        observed: None,
                        build_after: after,
                    },
                );
                assert_eq!(builder.build().err().unwrap(), expected);
                assert!(
                    builder.record_charge() > after[0]
                        && builder.spool_charge() > after[1]
                        && builder.work_steps() > after[3]
                );
                assert_eq!(builder.output_charge(), after[2]);
            },
        )
        .unwrap();
    }
}

fn text_reference_data(explicit: bool) -> (Value, String) {
    let mut data = description_data();
    let paragraph = data["document"]["blocks"][0]["items"][0]["blocks"][0].clone();
    let mut heading = paragraph.clone();
    heading["kind"] = "heading".into();
    heading["level"] = 1.into();
    heading["anchor_id"] = "navigation.text.label".into();
    let text = heading["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["kind"] == "text")
        .unwrap()
        .clone();
    let span = heading["span"].clone();
    heading["children"] = json!([
        {"kind":"strong","node_id":0,"span":span,"children":[text]},
        {"kind":"hard_break","node_id":0,"span":span},
        {"kind":"emphasis","node_id":0,"span":span,"children":[text]},
        {"kind":"soft_break","node_id":0,"span":span},text
    ]);
    let mut reference = paragraph;
    reference["children"] = json!([{"kind":"reference","node_id":0,"span":span,"target":"navigation.text.label","format":"text"}]);
    data["document"]["blocks"] = json!([reference, heading]);
    let mut rule = data["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["style_id"] == "paragraph-text")
        .unwrap()
        .clone();
    rule["style_id"] = "navigation-heading".into();
    rule["selector"] = "heading".into();
    rule["source_order"] = data["style_sheet"]["rules"]
        .as_array()
        .unwrap()
        .len()
        .into();
    data["style_sheet"]["rules"]
        .as_array_mut()
        .unwrap()
        .push(rule);
    semantic_renumber(&mut data["document"], &mut 0);
    let label = if explicit {
        "Proof ".repeat(20)
    } else {
        "Proof Proof Proof".into()
    };
    if explicit {
        let owner = data["document"]["blocks"][1]["node_id"].clone();
        data["outline"]["entries"] = json!([{"outline_id":0,"parent_outline_id":null,"level":1,"destination":"navigation.text.label","label":label,"source_node_id":owner,"source_kind":"heading"}]);
    }
    (data, label)
}

#[test]
fn book_v2_column_navigation_text_references_use_exact_outline_or_heading_only_labels() {
    let caps = display_caps();
    for explicit in [false, true] {
        let (data, label) = text_reference_data(explicit);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        navigation_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, builder, observed| {
                let facts = inspect_navigation(bundle.navigation());
                assert_eq!(facts.links.len(), 1);
                assert_eq!(facts.outline.len(), usize::from(explicit));
                let reference_owner =
                    bundle.display().navigation_source().paragraphs()[0].items()[0].owner();
                let generated = bundle
                    .display()
                    .text()
                    .draws()
                    .iter()
                    .filter(|d| d.owner() == reference_owner)
                    .map(|d| d.exact_text())
                    .collect::<String>();
                assert_eq!(generated, label);
                if explicit {
                    assert!(label.len() > 96);
                }
                let n = bundle.navigation();
                navigation_probes::emit(
                    &format!("navigation-text-reference-{explicit}"),
                    Some(n),
                    bundle.relations(),
                    None,
                    &caps,
                    navigation_probes::Context {
                        scope: "navigation_driver",
                        parent_scope: "relation_driver",
                        maximum: MAXIMUM,
                        constructors: bundle.constructor_charges(),
                        font_units: bundle.marked().paints().reservation_sizes(),
                        first_bounds_size: builder.reservation_sizes()[0],
                        encoded: Some(bundle.encoded_prefix()),
                        marked: Some(bundle.marked_prefix()),
                        relations: Some(bundle.relations_prefix()),
                        ancestor: Some(bundle.relations_prefix()),
                        parent_caller_before: Some(bundle.caller_before()),
                        caller_before: Some(bundle.caller_before()),
                        observed: Some(observed),
                        build_after: [
                            n.record_charge(),
                            n.spool_charge(),
                            n.output_charge(),
                            n.work_steps(),
                        ],
                    },
                );
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_navigation_named_lists_empty_notes_and_unreferenced_sources_keep_real_destinations(
) {
    let caps = display_caps();
    for ordered in [false, true] {
        let mut data = plain_data("Pro", true, false);
        wrap_list(&mut data["document"]["blocks"], ordered);
        list_font_rule(&mut data);
        balance(&mut data, true);
        semantic_renumber(&mut data["document"], &mut 0);
        let facts = run_document(
            &format!("navigation-original-list-{ordered}"),
            data,
            "list",
            &caps,
        );
        assert_eq!(
            facts
                .destinations
                .iter()
                .filter(|d| d.1[0] == "footnote")
                .count(),
            2
        );
        assert!(facts
            .destinations
            .iter()
            .any(|d| d.1[0] == "footnote" && d.2.is_none()));
    }
    for blanks in [1, 2] {
        for balanced in [false, true] {
            let facts = run_document(
                &format!("navigation-original-named-{blanks}-{balanced}"),
                named_display_data(
                    "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro",
                    blanks,
                    balanced,
                ),
                "body",
                &caps,
            );
            assert!(facts.pages.iter().take(blanks).all(|r| r.is_empty()));
        }
    }
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
        let facts = run_document(
            &format!("navigation-original-note-only-forced-{forced}"),
            data,
            "body",
            &caps,
        );
        assert!(facts.pages.len() > 1);
        let return_link = facts.links.last().unwrap();
        assert_eq!(
            return_link
                .rectangles
                .iter()
                .map(|&i| facts.rectangles[i].page)
                .collect::<BTreeSet<_>>()
                .len(),
            1
        );
        // The return Link belongs to the original note label. Later note
        // paragraphs retain their own unlinked original parents.
        assert!(facts.pages.iter().skip(1).all(|p| p.is_empty()));
        assert!(return_link
            .rectangles
            .iter()
            .all(|&i| facts.rectangles[i].column.is_none()));
    }
    for table in [false, true] {
        for balanced in [false, true] {
            let mut data = if table {
                table_data("common", "Pro")
            } else {
                plain_data("Pro", false, true)
            };
            if table {
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
            } else {
                let span = data["document"]["blocks"][0]["span"].clone();
                let force = json!({"kind":"page_break","node_id":0,"classes":[],"span":span});
                let blocks = data["document"]["blocks"].as_array_mut().unwrap();
                blocks.insert(0, force.clone());
                blocks.push(force);
            }
            balance(&mut data, balanced);
            semantic_renumber(&mut data["document"], &mut 0);
            let facts = run_document(
                &format!("navigation-original-zero-{table}-{balanced}"),
                data,
                "body",
                &caps,
            );
            assert!(facts.rectangles.is_empty() && facts.links.is_empty());
            assert!(facts.pages.iter().all(|p| p.is_empty()));
        }
    }
    let mut data = header_data(true, true);
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "unused-relation-table".into();
    fn unique(v: &mut Value) {
        match v {
            Value::Array(a) => {
                for v in a {
                    unique(v)
                }
            }
            Value::Object(o) => {
                if let Some(id) = o.get_mut("anchor_id").filter(|v| v.is_string()) {
                    *id = format!("unused-relations-{}", id.as_str().unwrap()).into();
                }
                for v in o.values_mut() {
                    unique(v)
                }
            }
            _ => (),
        }
    }
    unique(&mut unused);
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    anchors(&mut data);
    let facts = run_document(
        "navigation-original-unused-nested-definition",
        data,
        "vector",
        &caps,
    );
    assert!(facts
        .destinations
        .iter()
        .any(|d| d.1[0] == "footnote" && d.2.is_none()));
}

#[test]
fn book_v2_column_navigation_original_alias_figures_and_balanced_six_streams_keep_owner_positions()
{
    let caps = display_caps();
    for (kind, bytes) in [("png", PNG), ("jpeg", JPEG), ("svg", PAINT_CURVES)] {
        let mut data=super::super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
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
        let facts = input_navigation(
            &format!("navigation-original-image-alias-{kind}"),
            &input,
            &caps,
        );
        assert!(facts.destinations.iter().filter(|d| d.2.is_some()).count() >= 2);
    }
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let facts = input_navigation("navigation-original-balanced-six-stream", &input, &caps);
    assert!(facts.rectangles.iter().any(|r| r.column.is_none()));
    for (case,data,font) in [
        ("navigation-original-real-gsub",plain_data("ABA fi X D E f i",false,false),Some(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf")).as_slice())),
        ("navigation-original-positive-settlement",plain_data("Pro",false,false),None),
    ] {
        let root=Root::new();let input=display_input(&root,data,font,&caps);let facts=input_navigation(case,&input,&caps);assert!(facts.links.is_empty());
    }
}

#[test]
fn book_v2_column_navigation_exact_four_caps_and_each_vector_reservation_keep_accepted_prefixes() {
    let caps = display_caps();
    let (mut data, _) = text_reference_data(true);
    let plain = plain_data("Pro", true, false);
    let note = plain["document"]["footnotes"][0].clone();
    let span = data["document"]["blocks"][0]["span"].clone();
    data["document"]["blocks"][0]["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":note["footnote_id"]}));
    data["document"]["footnotes"] = json!([note]);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        let body = m["body"].clone();
        let bottom = m["height"].as_i64().unwrap();
        m["footnote"] =
            json!({"x":body["x"],"y":bottom-3_000_000,"width":body["width"],"height":2_000_000});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    data["outline"]["entries"][0]["source_node_id"] =
        data["document"]["blocks"][1]["node_id"].clone();
    let root = Root::new();
    let input = display_input(&root, data, Some(DISPLAY_LIST_FONT), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    relation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
        let source = bundle.relations();
        let display = bundle.display();
        let view = display.navigation_source();
        let floor = [
            source.record_charge(),
            source.spool_charge(),
            source.output_charge(),
            source.work_steps(),
        ];
        let run = |prior: [u64; 4], maximum: u64, limits: &M4EffectiveResourceLimits| {
            let (mut r, mut s, mut o, mut w) = (0, 0, 0, 0);
            let made = Navigation::new_counted(
                source, limits, maximum, prior[0], prior[1], prior[2], prior[3], &mut r, &mut s,
                &mut o, &mut w,
            );
            let constructor = [r, s, o, w];
            let mut b = match made {
                Ok(b) => b,
                Err(e) => {
                    return EncodedAttempt {
                        constructor,
                        after: constructor,
                        fingerprint: None,
                        error: Some(format!("{e:?}")),
                    }
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
        let base = run([0; 4], MAXIMUM, &caps);
        assert!(base.error.is_none(), "{base:?}");
        let mut diagnostic = Navigation::new(source, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
        let units = diagnostic.reservation_sizes()[0];
        let built = diagnostic.build().unwrap();
        inspect_navigation(&built);
        let counts = [
            built.destinations().len(),
            built.links().len(),
            source.source().source().source().groups().len(),
            source.source().source().pages().len(),
            view.outline().len(),
            source.source().nodes().len(),
            source.source().nodes().len(),
        ];
        let sizes = [
            std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationDestination>(),
            std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationLink>(),
            std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationRectangle>(),
            std::mem::size_of::<std::ops::Range<usize>>(),
            std::mem::size_of::<typaxis_pdf::book_v2::BookV2NavigationOutline>(),
            std::mem::size_of::<Option<usize>>(),
            units,
        ];
        assert!(counts.iter().all(|&n| n > 0));
        assert_eq!(
            base.after[0] - floor[0],
            1 + counts.iter().sum::<usize>() as u64
        );
        assert_eq!(
            base.after[1] - floor[1],
            counts
                .iter()
                .zip(sizes)
                .map(|(&n, s)| (n * s) as u64)
                .sum::<u64>()
        );
        assert_eq!(base.after[2], floor[2]);
        for (axis, cap, label) in [
            (0, caps.base().get().max_fragments, "Records"),
            (1, caps.base().get().max_spool_bytes, "Spool"),
        ] {
            let delta = base.after[axis] - floor[axis];
            let mut prior = [0; 4];
            prior[axis] = cap - delta;
            let exact = run(prior, MAXIMUM, &caps);
            assert_eq!(exact.error, None);
            assert_eq!(exact.after[axis], cap);
            assert_eq!(exact.fingerprint, base.fingerprint);
            prior[axis] += 1;
            let short = run(prior, MAXIMUM, &caps);
            assert_eq!(
                short.error.as_deref(),
                Some(if label == "Records" {
                    "Resource(Records)"
                } else {
                    "Resource(Spool)"
                })
            );
            assert!(short.after[axis] <= cap);
        }
        let cap = caps.base().get().max_output_bytes;
        assert_eq!(run([0, 0, cap, 0], MAXIMUM, &caps).after[2], cap);
        let over = run([0, 0, cap + 1, 0], MAXIMUM, &caps);
        assert_eq!(over.error.as_deref(), Some("Resource(Output)"));
        assert_eq!(over.constructor, [floor[0], floor[1], cap + 1, floor[3]]);
        let exact = run([0; 4], base.after[3], &caps);
        assert_eq!(exact.fingerprint, base.fingerprint);
        let short = run([0; 4], base.after[3] - 1, &caps);
        assert_eq!(short.error.as_deref(), Some("Resource(Work)"));
        assert_eq!(&short.after[..3], &base.after[..3]);
        let changed = changed_caps(&caps, |c| c.max_fragments += 1);
        let mismatch = run([0; 4], MAXIMUM, &changed);
        assert_eq!(mismatch.error.as_deref(), Some("Resource(Identity)"));
        assert_eq!(mismatch.constructor, floor);
        let cap = caps.base().get().max_fragments;
        assert_eq!(run([cap - 1, 0, 0, 0], MAXIMUM, &caps).constructor[0], cap);
        let short = run([cap, 0, 0, 0], MAXIMUM, &caps);
        assert_eq!(short.error.as_deref(), Some("Resource(Records)"));
        assert_eq!(short.constructor[0], cap);
        let short = run([0; 4], floor[3] - 1, &caps);
        assert_eq!(short.error.as_deref(), Some("Resource(Work)"));
        assert_eq!(
            short.constructor,
            [floor[0] + 1, floor[1], floor[2], floor[3]]
        );
        let census = view
            .paragraphs()
            .iter()
            .map(|p| 1 + p.items().len() as u64)
            .sum::<u64>()
            + source.notes().len() as u64;
        let (mut records, mut spool) = (1u64, 0u64);
        for k in 0..7 {
            let refused = run([0; 4], floor[3] + census + k as u64, &caps);
            records += counts[k] as u64;
            spool += (counts[k] * sizes[k]) as u64;
            assert_eq!(
                refused.error.as_deref(),
                Some("Resource(Work)"),
                "reservation{k}:{refused:?}"
            );
            assert_eq!(
                refused.after,
                [
                    floor[0] + records,
                    floor[1] + spool,
                    floor[2],
                    floor[3] + census + k as u64
                ]
            );
            for (axis, cap, label, total, amount) in [
                (
                    0,
                    caps.base().get().max_fragments,
                    "Resource(Records)",
                    records,
                    counts[k] as u64,
                ),
                (
                    1,
                    caps.base().get().max_spool_bytes,
                    "Resource(Spool)",
                    spool,
                    (counts[k] * sizes[k]) as u64,
                ),
            ] {
                let mut prior = [0; 4];
                prior[axis] = cap - total + 1;
                let rejected = run(prior, MAXIMUM, &caps);
                assert_eq!(
                    rejected.error.as_deref(),
                    Some(label),
                    "reservation{k} axis{axis}:{rejected:?}"
                );
                assert_eq!(rejected.after[axis], cap - amount + 1);
            }
        }
        let a = diagnostic.build().unwrap();
        assert_eq!(a.fingerprint(), built.fingerprint());
        assert_eq!(
            a.record_charge() - built.record_charge(),
            counts.iter().sum::<usize>() as u64
        );
        assert_eq!(
            a.spool_charge() - built.spool_charge(),
            base.after[1] - floor[1]
        );
        assert_eq!(a.output_charge(), built.output_charge());
        assert!(a.work_steps() > built.work_steps());
        let mut fail = Navigation::new(source, &caps, base.after[3] - 1, 0, 0, 0, 0).unwrap();
        assert!(fail.build().is_err());
        let before = [
            fail.record_charge(),
            fail.spool_charge(),
            fail.output_charge(),
            fail.work_steps(),
        ];
        assert!(fail.build().is_err());
        assert_eq!(
            [
                fail.record_charge(),
                fail.spool_charge(),
                fail.output_charge(),
                fail.work_steps()
            ],
            before
        );
    })
    .unwrap();
}

#[test]
fn book_v2_column_navigation_driver_exact_caps_callback_failure_and_retries_never_refund_prefixes()
{
    let caps = display_caps();
    let data = description_data();
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut peak = 0;
    navigation_drive(&input, &caps, MODE, &mut warm, 4, |bundle, _, _| {
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
            let result = navigation_drive(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |bundle, builder, observed| {
                    delivered = true;
                    inspect_navigation(bundle.navigation());
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
                    let n = bundle.navigation();
                    navigation_probes::emit(
                        &format!("navigation-driver-exact-{axis}"),
                        Some(n),
                        bundle.relations(),
                        None,
                        &limits,
                        navigation_probes::Context {
                            scope: "navigation_driver",
                            parent_scope: "relation_driver",
                            maximum,
                            constructors: bundle.constructor_charges(),
                            font_units: bundle.marked().paints().reservation_sizes(),
                            first_bounds_size: builder.reservation_sizes()[0],
                            encoded: Some(bundle.encoded_prefix()),
                            marked: Some(bundle.marked_prefix()),
                            relations: Some(bundle.relations_prefix()),
                            ancestor: Some(bundle.relations_prefix()),
                            parent_caller_before: Some(bundle.caller_before()),
                            caller_before: Some(bundle.caller_before()),
                            observed: Some(observed),
                            build_after: [
                                n.record_charge(),
                                n.spool_charge(),
                                n.output_charge(),
                                n.work_steps(),
                            ],
                        },
                    );
                },
            );
            if short {
                assert!(result.is_err(), "axis{axis}");
                assert!(!delivered);
                let before = paint_observation(budget.observation());
                assert!(
                    navigation_drive(&input, &limits, MODE, &mut budget, 4, |_, _, _| delivered =
                        true)
                    .is_err()
                );
                assert!(!delivered);
                let after = paint_observation(budget.observation());
                for i in [0, 1, 3] {
                    assert!(after[i] >= before[i]);
                }
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
    let mut accepted = [0; 4];
    let refusal: Result<(), _> =
        navigation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, builder, _| {
            let n = builder.build().unwrap();
            inspect_navigation(&n);
            assert_eq!(n.fingerprint(), bundle.navigation().fingerprint());
            assert_eq!(n.output_charge(), bundle.navigation().output_charge());
            accepted = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            Err("navigation callback refusal")
        })
        .unwrap();
    assert_eq!(refusal, Err("navigation callback refusal"));
    assert_eq!(paint_observation(budget.observation()), accepted);
    navigation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            assert_eq!(bundle.caller_before(), accepted);
            assert!(
                observed.record_charge() > accepted[0]
                    && observed.spool_charge() > accepted[1]
                    && observed.work_steps() > accepted[3]
            );
            assert_eq!(
                observed.output_charge(),
                bundle.relations_prefix().output_charge()
            );
            inspect_navigation(bundle.navigation());
        },
    )
    .unwrap();
    let maximum = accepted[3] - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
    let mut callback_failure = [0; 4];
    navigation_drive(&input, &caps, MODE, &mut budget, 4, |bundle, builder, _| {
        assert!(matches!(
            builder.build(),
            Err(NavigationError::Resource(RelationError::Work))
        ));
        callback_failure = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.output_charge(),
            builder.work_steps(),
        ];
        assert_eq!(
            callback_failure[2],
            bundle.relations_prefix().output_charge()
        );
    })
    .unwrap();
    assert_eq!(paint_observation(budget.observation()), callback_failure);
    assert!(callback_failure[3] <= maximum);
    let maximum = totals[3] - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
    let mut delivered = false;
    let error = navigation_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
        delivered = true
    })
    .unwrap_err();
    assert!(
        matches!(
            error,
            crate::book_v2_resources::BookV2ConvergenceError::Stage {
                stage: "column navigation geometry",
                ..
            }
        ),
        "{error:?}"
    );
    assert!(!delivered);
    assert_eq!(budget.observation().record_charge(), totals[0]);
    assert_eq!(budget.observation().spool_charge(), totals[1]);
    assert!(budget.observation().work_steps() <= maximum);
}

fn manual_navigation(
    case: &str,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
) {
    let mut constructors = [[0; 4]; 13];
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
    let mut navigation = Navigation::new(&tree, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[12] = [
        navigation.record_charge(),
        navigation.spool_charge(),
        navigation.output_charge(),
        navigation.work_steps(),
    ];
    let n = navigation.build().unwrap();
    inspect_navigation(&n);
    navigation_probes::emit(
        case,
        Some(&n),
        &tree,
        None,
        caps,
        navigation_probes::Context {
            scope,
            parent_scope: scope,
            maximum: MAXIMUM,
            constructors,
            font_units: units,
            first_bounds_size: navigation.reservation_sizes()[0],
            encoded: observed,
            marked: observed,
            relations: None,
            ancestor: observed,
            parent_caller_before: None,
            caller_before: None,
            observed: None,
            build_after: [
                n.record_charge(),
                n.spool_charge(),
                n.output_charge(),
                n.work_steps(),
            ],
        },
    );
}

#[test]
fn book_v2_column_navigation_manual_original_source_callbacks_keep_native_vector_and_retained_gid71_roots(
) {
    let caps = display_caps();
    for kind in ["native", "vector"] {
        let data=super::super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,true,true);
        let root = Root::new();
        let input = if kind == "native" {
            let source = data["text_buffers"][0]["utf8"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec();
            native_input_with_source(&root, data, &source, &caps)
        } else {
            vector_input(&root, data, &caps)
        };
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        math_drive(&input, &caps, MODE, &mut budget, |source, _, observed| {
            let mut b =
                BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap();
            let display = b.build().unwrap();
            manual_navigation(
                &format!("navigation-manual-source-{kind}"),
                Some(observed),
                "source_callback",
                &display,
                &caps,
            );
        })
        .unwrap();
    }
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
        assert!(original.contains(&59) && !original.contains(&71) && repeated.contains(&71));
        manual_navigation(
            "navigation-context-retained-cut",
            None,
            "retained_source_cut",
            display,
            caps,
        );
    });
}

#[test]
#[ignore = "requires TYPAXIS_HARANO_FONT pointing to original HaranoAjiMincho-Regular.otf SHA256 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"]
fn book_v2_column_navigation_original_harano_cid_cff_source_geometry() {
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
    let facts = input_navigation("navigation-original-harano-cid-cff", &input, &caps);
    assert!(!facts.links.is_empty());
}

fn wrap_paragraph_links(v: &mut Value) {
    fn has_reference(v: &Value) -> bool {
        match v {
            Value::Object(o) => {
                o.get("kind").is_some_and(|v| v == "footnote_reference")
                    || o.values().any(has_reference)
            }
            Value::Array(a) => a.iter().any(has_reference),
            _ => false,
        }
    }
    match v {
        Value::Array(a) => {
            for v in a {
                wrap_paragraph_links(v)
            }
        }
        Value::Object(o) => {
            if o.get("kind").is_some_and(|v| v == "paragraph") && !has_reference(&o["children"]) {
                let span = o["span"].clone();
                let children = o["children"].clone();
                o.insert("children".into(),json!([{"kind":"link","node_id":0,"span":span,"target":{"kind":"uri","uri":"https://example.invalid/actual-source"},"children":children}]));
            } else {
                for v in o.values_mut() {
                    wrap_paragraph_links(v)
                }
            }
        }
        _ => (),
    }
}

#[test]
fn book_v2_column_navigation_link_rectangles_cover_native_rules_real_headers_and_nested_cells_without_artifact_merges(
) {
    let caps = display_caps();
    for vector in [false, true] {
        for notes in [false, true] {
            let mut data = header_data(vector, notes);
            wrap_paragraph_links(&mut data["document"]);
            semantic_renumber(&mut data["document"], &mut 0);
            let facts = run_document(
                &format!("navigation-linked-real-header-{vector}-{notes}"),
                data,
                if vector { "vector" } else { "native" },
                &caps,
            );
            assert!(facts.links.len() > 2 && facts.rectangles.len() > 2);
        }
    }
    let mut data = fraction_data(false);
    wrap_paragraph_links(&mut data["document"]);
    semantic_renumber(&mut data["document"], &mut 0);
    let facts = run_document(
        "navigation-linked-native-fraction-rule",
        data,
        "native",
        &caps,
    );
    assert_eq!(facts.links.len(), 1);
    assert!(facts.rectangles.len() > 0);
    for (mode, height) in [
        ("span", 256),
        ("nested-body-span", 544),
        ("nested-body-caption", 512),
        ("forced", 256),
    ] {
        let mut data = table_data(mode, "Pro Pro Pro Pro Pro Pro Pro");
        for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
            m["body"]["height"] = (height * 65536).into();
        }
        wrap_paragraph_links(&mut data["document"]);
        semantic_renumber(&mut data["document"], &mut 0);
        let facts = run_document(
            &format!("navigation-linked-table-{mode}"),
            data,
            "body",
            &caps,
        );
        assert!(facts.links.len() > 2 && facts.rectangles.len() > 2);
        assert!(facts.rectangles.iter().all(|r| r.column.is_some()));
    }
}
#[test]
fn book_v2_column_navigation_nonpainting_anchor_targets_use_actual_selected_line_baselines() {
    let caps = display_caps();
    let mut data = plain_data("Pro", false, true);
    let plain = plain_data("Pro", false, false);
    let mut p = plain["document"]["blocks"][0].clone();
    let span = p["span"].clone();
    let children = p["children"].clone();
    p["children"] = json!([{"kind":"link","node_id":0,"span":span,"target":{"kind":"internal","anchor_id":"empty-column-width-0"},"children":children}]);
    data["document"]["blocks"].as_array_mut().unwrap().push(p);
    semantic_renumber(&mut data["document"], &mut 0);
    let facts = run_document("navigation-empty-anchor-target", data, "body", &caps);
    assert_eq!(facts.destinations.len(), 6);
    assert!(facts.destinations.iter().all(|d| d.2.is_some()));
    assert_eq!(facts.links.len(), 1);
    assert!(!facts.links[0].rectangles.is_empty());
    let first = facts.destinations[0].2.unwrap();
    assert!(first[3] > 0);
    assert!(facts
        .rectangles
        .iter()
        .all(|r| r.fragment != first[1] as usize));
}

#[test]
fn book_v2_column_navigation_remaining_original_relation_packages_keep_selected_source_semantics() {
    let caps = display_caps();
    input_navigation(
        "navigation-original-caller-note-source",
        &display_input(&Root::new(), plain_data("Pro", true, false), None, &caps),
        &caps,
    );
    for balanced in [false, true] {
        let mut data = plain_data("Pro", true, false);
        anchors(&mut data);
        wrap_list(&mut data["document"]["blocks"], true);
        list_font_rule(&mut data);
        balance(&mut data, balanced);
        renumber(&mut data);
        let facts = run_document(
            &format!("navigation-original-list-notes-{balanced}"),
            data,
            "list",
            &caps,
        );
        assert!(!facts.links.is_empty());
    }
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
    run_document(
        "navigation-original-multi-head-intervals",
        data,
        "body",
        &caps,
    );
    let mut data = table_data("common", "Pro");
    let plain = plain_data("Pro", true, false);
    let mut note = plain["document"]["footnotes"][0].clone();
    note["footnote_id"] = "relation-term-note".into();
    data["document"]["footnotes"] = json!([note]);
    let p = &mut data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0];
    let span = p["span"].clone();
    p["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"relation-term-note"}));
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        let body = m["body"].clone();
        let bottom = m["height"].as_i64().unwrap();
        m["footnote"] =
            json!({"x":body["x"],"y":bottom-3_000_000,"width":body["width"],"height":2_000_000});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    let facts = run_document("navigation-original-cell-reference", data, "list", &caps);
    assert!(!facts.links.is_empty());
    let mut data=super::super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data("vector",true,false);
    let mut unused = data["document"]["footnotes"][0].clone();
    unused["footnote_id"] = "marked-unused-rich".into();
    data["document"]["footnotes"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    anchors(&mut data);
    let facts = run_document("navigation-original-unused-rich", data, "vector", &caps);
    assert!(facts
        .destinations
        .iter()
        .any(|d| d.1[0] == "footnote" && d.2.is_none()));
    const FONT:&[u8]=include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"));
    let mut data = plain_data(&"AB ".repeat(48), false, false);
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["width"] = (660 * 65536).into();
        m["trim"]["width"] = (660 * 65536).into();
        m["body"]["width"] = (610 * 65536).into();
    }
    let root = Root::new();
    let input = display_input(&root, data, Some(FONT), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    navigation_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            assert!(
                bundle.marked().content().output_charge()
                    < bundle.marked().content().source().output_charge()
            );
            assert_eq!(
                bundle.navigation().output_charge(),
                bundle.marked().content().output_charge()
            );
            inspect_navigation(bundle.navigation());
            let n = bundle.navigation();
            navigation_probes::emit(
                "navigation-original-negative-settlement",
                Some(n),
                bundle.relations(),
                None,
                &caps,
                navigation_probes::Context {
                    scope: "navigation_driver",
                    parent_scope: "relation_driver",
                    maximum: MAXIMUM,
                    constructors: bundle.constructor_charges(),
                    font_units: bundle.marked().paints().reservation_sizes(),
                    first_bounds_size: builder.reservation_sizes()[0],
                    encoded: Some(bundle.encoded_prefix()),
                    marked: Some(bundle.marked_prefix()),
                    relations: Some(bundle.relations_prefix()),
                    ancestor: Some(bundle.relations_prefix()),
                    parent_caller_before: Some(bundle.caller_before()),
                    caller_before: Some(bundle.caller_before()),
                    observed: Some(observed),
                    build_after: [
                        n.record_charge(),
                        n.spool_charge(),
                        n.output_charge(),
                        n.work_steps(),
                    ],
                },
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_navigation_original_header_reference_has_one_semantic_link_across_faithful_repeats(
) {
    let caps = display_caps();
    let mut data = table_data("common", "Pro");
    let plain = plain_data("Pro", true, false);
    let mut note = plain["document"]["footnotes"][0].clone();
    note["footnote_id"] = "header-relation-note".into();
    data["document"]["footnotes"] = json!([note]);
    let table = &mut data["document"]["blocks"][0];
    let p = &mut table["head"][0]["cells"][0]["blocks"][0];
    let span = p["span"].clone();
    p["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":"header-relation-note"}));
    let row = table["body"][0].clone();
    table["body"] = vec![row; 40].into();
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["body"]["height"] = (1024 * 65536).into();
        m["body"]["width"] = (290 * 65536).into();
        let body = m["body"].clone();
        let bottom = m["height"].as_i64().unwrap();
        m["footnote"] =
            json!({"x":body["x"],"y":bottom-9_000_000,"width":body["width"],"height":128*65536});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    let facts = run_document("navigation-original-header-reference", data, "list", &caps);
    assert_eq!(facts.links.len(), 2);
    assert_eq!(facts.links[0].rectangles.len(), 1);
    assert!(facts.pages.len() > 1);
}

#[test]
fn book_v2_column_navigation_one_link_keeps_text_native_rule_vector_and_descendant_boxes_across_all_columns(
) {
    let caps = display_caps();
    for native in [false, true] {
        for columns in [2, 3] {
            let mut data = if native {
                fraction_data(false)
            } else {
                super::super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data("vector",false,false)
            };
            let p = &mut data["document"]["blocks"][0]["blocks"][0];
            let span = p["span"].clone();
            let original = p["children"].as_array().unwrap().clone();
            let mut children = original
                .iter()
                .filter(|c| c["kind"] == "anchor")
                .cloned()
                .collect::<Vec<_>>();
            let atoms = original
                .iter()
                .filter(|c| c["kind"] != "anchor")
                .cloned()
                .collect::<Vec<_>>();
            // Keep the original math occurrence and its original source range.
            // Repeated trailing text keeps monotone parsed source positions.
            children.extend(atoms.iter().cloned());
            let tail = atoms.iter().rev().find(|v| v["kind"] == "text").unwrap();
            for iteration in 0..96 {
                children.push(json!({"kind":if iteration%2==0{"strong"}else{"emphasis"},"node_id":0,"span":tail["span"],"children":[tail]}));
                children.push(json!({"kind":"soft_break","node_id":0,"span":tail["span"]}));
            }
            p["children"] = json!([{"kind":"link","node_id":0,"span":span,"target":{"kind":"uri","uri":"https://example.invalid/multiple-components"},"children":children}]);
            for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
                m["width"] = (360 * 65536).into();
                m["trim"]["width"] = (360 * 65536).into();
                m["body"]["width"] = ((columns * 100 + (columns - 1) * 10) * 65536).into();
                m["body"]["height"] = (96 * 65536).into();
                m["column_layout"]["count"] = columns.into();
                m["column_layout"]["gap"] = (10 * 65536).into();
            }
            semantic_renumber(&mut data["document"], &mut 0);
            let facts = run_document(
                &format!("navigation-multiple-components-{native}-{columns}"),
                data,
                if native { "native" } else { "vector" },
                &caps,
            );
            assert_eq!(facts.links.len(), 1);
            assert!(
                facts
                    .rectangles
                    .iter()
                    .map(|r| r.page)
                    .collect::<BTreeSet<_>>()
                    .len()
                    > 1
            );
            assert_eq!(
                facts
                    .rectangles
                    .iter()
                    .filter(|r| r.page == 0)
                    .map(|r| r.column.unwrap())
                    .collect::<BTreeSet<_>>(),
                (0..columns as u16).collect::<BTreeSet<_>>()
            );
        }
    }
}

#[test]
fn book_v2_column_navigation_middle_hardbreak_and_empty_header_note_targets_remain_nonpainting() {
    let caps = display_caps();
    let mut data = plain_data("Pro", false, false);
    let p = &mut data["document"]["blocks"][0];
    let span = p["span"].clone();
    let text = p["children"][0].clone();
    p["children"] = json!([text,{"kind":"hard_break","node_id":0,"span":span},{"kind":"anchor","node_id":0,"span":span,"anchor_id":"middle.empty.line"},{"kind":"hard_break","node_id":0,"span":span},text]);
    semantic_renumber(&mut data["document"], &mut 0);
    let facts = run_document("navigation-middle-hardbreak-anchor", data, "body", &caps);
    assert_eq!(facts.destinations.len(), 1);
    assert!(facts.destinations[0].2.is_some());
    assert!(facts.rectangles.is_empty());
    let mut data = table_data("common", "Pro");
    let mut serial = 0;
    for row in data["document"]["blocks"][0]["head"]
        .as_array_mut()
        .unwrap()
    {
        for cell in row["cells"].as_array_mut().unwrap() {
            for p in cell["blocks"].as_array_mut().unwrap() {
                let span = p["span"].clone();
                p["children"] = json!([{"kind":"anchor","node_id":0,"span":span,"anchor_id":format!("empty-original-header-{serial}")}]);
                serial += 1;
            }
        }
    }
    let row = data["document"]["blocks"][0]["body"][0].clone();
    data["document"]["blocks"][0]["body"] = vec![row; 40].into();
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["body"]["height"] = (1024 * 65536).into();
        m["body"]["width"] = (290 * 65536).into();
    }
    let plain = plain_data("Pro", true, false);
    let mut note = plain["document"]["footnotes"][0].clone();
    let p = &mut note["blocks"][0];
    let span = p["span"].clone();
    p["children"] =
        json!([{"kind":"anchor","node_id":0,"span":span,"anchor_id":"empty-demanded-note"}]);
    let id = note["footnote_id"].clone();
    data["document"]["footnotes"] = json!([note]);
    let p = &mut data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][0];
    let span = p["span"].clone();
    p["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"footnote_reference","node_id":0,"span":span,"footnote_id":id}));
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        let body = m["body"].clone();
        let bottom = m["height"].as_i64().unwrap();
        m["footnote"] =
            json!({"x":body["x"],"y":bottom-9_000_000,"width":body["width"],"height":128*65536});
    }
    semantic_renumber(&mut data["document"], &mut 0);
    let facts = run_document(
        "navigation-empty-original-header-demanded-note",
        data,
        "list",
        &caps,
    );
    assert!(facts.pages.len() > 1);
    assert!(facts
        .destinations
        .iter()
        .filter(|d| d.1[0] == "anchor")
        .all(|d| d.2.is_some()));
}

#[test]
fn book_v2_column_navigation_page_reference_requires_authentic_counted_candidate_values() {
    let caps = display_caps();
    let mut data = number_reference_data();
    data["document"]["blocks"][0]["children"][0]["format"] = "page".into();
    let root = Root::new();
    let input = display_input(&root, data, None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut delivered = false;
    let error = navigation_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
        delivered = true
    })
    .unwrap_err();
    assert!(!delivered);
    assert!(
        matches!(
            error,
            crate::book_v2_resources::BookV2ConvergenceError::Stage {
                stage: "column line convergence",
                ..
            }
        ),
        "{error:?}"
    );
    assert!(format!("{error:?}").contains("PendingInline(\"reference\")"));
    assert!(format!("{error:?}").contains("owner: NodeId(2)"));
    eprintln!(
        "Column Page-reference dependency refusal: {error:?}; retained {:?}",
        paint_observation(budget.observation())
    );
}

#[test]
fn book_v2_column_navigation_original_formula_and_figure_targets_use_their_own_component_origins() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "svg"] {
        let mut data=super::super::super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
        let alias = kind == "svg";
        if alias {
            let mut image = data["resources"]["images"][0].clone();
            image["image_id"] = 1.into();
            data["resources"]["images"]
                .as_array_mut()
                .unwrap()
                .push(image);
        }
        fn enclose(v: &mut Value, serial: &mut usize, alias: bool) {
            if matches!(
                v["kind"].as_str(),
                Some("display_math" | "math_vector_block" | "figure" | "vector_figure")
            ) {
                if alias && matches!(v["kind"].as_str(), Some("figure" | "vector_figure")) {
                    v["image_id"] = (*serial % 2).into();
                }
                let block = v.take();
                let span = block["span"].clone();
                let id = *serial;
                *serial += 1;
                *v = json!({"kind":"semantic_container","node_id":0,"classes":[],"semantic_kind":"solution","anchor_id":format!("own-component-target-{id}"),"span":span,"blocks":[block]});
                return;
            }
            match v {
                Value::Array(a) => {
                    for v in a {
                        enclose(v, serial, alias)
                    }
                }
                Value::Object(o) => {
                    for v in o.values_mut() {
                        if v.is_array() || v.is_object() {
                            enclose(v, serial, alias)
                        }
                    }
                }
                _ => (),
            }
        }
        let mut count = 0;
        enclose(&mut data["document"], &mut count, alias);
        semantic_renumber(&mut data["document"], &mut 0);
        let facts = run_document(
            &format!("navigation-own-component-target-{kind}"),
            data,
            kind,
            &caps,
        );
        assert!(count >= 3);
        assert_eq!(facts.destinations.len(), count);
        assert!(facts.destinations.iter().all(|d| d.2.is_some()));
        assert_eq!(
            facts
                .destinations
                .iter()
                .map(|d| d.0)
                .collect::<BTreeSet<_>>()
                .len(),
            count
        );
        assert!(facts.links.is_empty());
    }
}
