use super::*;
use typaxis_pdf::book_v2::{
    BookV2ColumnMarkedContent, BookV2ColumnMarkedContentBuilder, BookV2ColumnMarkedScopeBuilder,
    BookV2ColumnSourceStructure, BookV2ColumnSourceStructureBuilder,
    BookV2MarkedArtifact as Artifact, BookV2MarkedRole as Role, BookV2SemanticAnchorRole,
};
use typaxis_syntax::book_v2::BookV2StructureKey as Key;

#[path = "book_v2_column_marked_continuation_tests.rs"]
mod continuation_tests;
#[path = "book_v2_column_marked_probes.rs"]
mod marked_probes;
#[path = "book_v2_column_relation_tests.rs"]
mod relation_tests;

#[derive(Clone, Debug, Eq, PartialEq)]
struct OriginalScope {
    owner: Option<typaxis_core::NodeId>,
    page: u32,
    fragment: usize,
    role: Role,
    artifact: Option<Artifact>,
    alternative: Option<String>,
    actual: Option<String>,
    whole: String,
    paints: std::ops::Range<usize>,
}
fn original_scopes(
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
) -> Vec<OriginalScope> {
    let mut scopes: Vec<OriginalScope> = Vec::new();
    for (paint, kind) in display.paints().iter().copied().enumerate() {
        let (owner, page, fragment, role, repeated, alternative, actual, whole) = match kind {
            Paint::PageRegionText { .. } => {
                panic!("running region outside the legacy Column fixture")
            }
            Paint::Text(i) => {
                let d = &display.text().draws()[i];
                (
                    Some(d.owner()),
                    d.page_index(),
                    d.fragment_index(),
                    Role::Text,
                    d.repeated_header(),
                    None,
                    None,
                    d.exact_text().to_owned(),
                )
            }
            Paint::Marker(i) => {
                let d = &display.markers().draws()[i];
                (
                    Some(d.source().owner()),
                    d.fragment().fragment().page_index(),
                    d.fragment_index(),
                    Role::Label,
                    d.repeated_header(),
                    None,
                    None,
                    d.source().utf8().to_owned(),
                )
            }
            Paint::EquationNumber(i) => {
                let d = &display.numbers().draws()[i];
                (
                    Some(d.placement().geometry().owner()),
                    d.placement().geometry().page_index(),
                    d.placement().geometry().fragment_index() as usize,
                    Role::EquationNumber,
                    d.placement().repeated_header(),
                    None,
                    None,
                    d.shape().text().to_owned(),
                )
            }
            Paint::Math(i) => {
                let d = &display.math().draws()[i];
                let (alt, actual) = match d.terminal().source() {
                    BookV2BodyMathSource::Native(n) => (
                        n.source().domain().speech.as_str(),
                        n.source().domain().speech.as_str(),
                    ),
                    BookV2BodyMathSource::Vector(v) => (
                        v.source().alternative().alternative(),
                        v.source().alternative().resolved_actual_text().unwrap(),
                    ),
                };
                (
                    Some(d.terminal().source().owner()),
                    d.terminal().page_index(),
                    d.terminal().fragment_index(),
                    Role::Formula,
                    d.terminal().repeated_header(),
                    Some(alt.to_owned()),
                    Some(actual.to_owned()),
                    String::new(),
                )
            }
            Paint::Image(i) => {
                let d = &display.images().draws()[i];
                let (alt, actual) = match d.source() {
                    BookV2ImageSource::Figure(f) => (f.source().alternative(), None),
                    BookV2ImageSource::Vector(v) => (
                        v.source().alternative().alternative(),
                        v.source().alternative().authored_actual_text(),
                    ),
                };
                (
                    Some(d.source().owner()),
                    d.fragment().fragment().page_index(),
                    d.fragment_index(),
                    Role::Figure,
                    d.repeated_header(),
                    Some(alt.to_owned()),
                    actual.map(str::to_owned),
                    String::new(),
                )
            }
            Paint::FootnoteSeparator(i) => {
                let d = &display.markers().separators()[i];
                (
                    None,
                    d.page_index(),
                    d.before_fragment_index(),
                    Role::Separator,
                    false,
                    None,
                    None,
                    String::new(),
                )
            }
        };
        let artifact = if repeated {
            Some(Artifact::RepeatedHeader)
        } else if role == Role::Separator {
            Some(Artifact::FootnoteSeparator)
        } else {
            None
        };
        let current = OriginalScope {
            owner,
            page,
            fragment,
            role,
            artifact,
            alternative,
            actual,
            whole,
            paints: paint..paint + 1,
        };
        if role == Role::Text {
            if let Some(prior) = scopes.last_mut() {
                if (
                    prior.owner,
                    prior.page,
                    prior.fragment,
                    prior.role,
                    prior.artifact,
                ) == (owner, page, fragment, role, artifact)
                {
                    prior.paints.end = paint + 1;
                    prior.whole.push_str(&current.whole);
                    continue;
                }
            }
        }
        scopes.push(current);
    }
    scopes
}
fn replacement_token(text: &str) -> String {
    format!(
        "<FEFF{}>",
        text.encode_utf16()
            .map(|c| format!("{c:04X}"))
            .collect::<String>()
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceNode {
    key: (u32, String),
    parent: Option<(u32, String)>,
    role: String,
    language: String,
    semantic: Option<String>,
    span: Option<[u32; 3]>,
    alternative: Option<String>,
    cell: Option<[u32; 6]>,
}
fn original_source_nodes(document: &Value) -> Vec<SourceNode> {
    fn add(
        v: &Value,
        kind: &str,
        parent: Option<(u32, String)>,
        language: &str,
        slot: &str,
        role: Option<&str>,
        cell: Option<[u32; 6]>,
        nodes: &mut Vec<SourceNode>,
    ) -> SourceNode {
        let role = role.map(str::to_owned).unwrap_or_else(|| {
            match kind {
                "document" => "Document",
                "paragraph" => "P",
                "description_list" | "list" => "L",
                "description_item" | "list_item" => "LI",
                "description_term" => "Lbl",
                "table" => "Table",
                "table_row" => "TR",
                "figure" | "vector_figure" | "inline_vector" => "Figure",
                "inline_math" | "display_math" | "math_vector" | "math_vector_block" => "Formula",
                "text" | "equation_number" => "Span",
                "emphasis" => "Em",
                "strong" => "Strong",
                "link" => "Link",
                "reference" | "footnote_reference" => "Reference",
                "footnote_definition" => "Note",
                "semantic_container" => {
                    if v["semantic_kind"] == "quote" {
                        "BlockQuote"
                    } else {
                        "Sect"
                    }
                }
                "heading" => return format!("H{}", v["level"]),
                _ => panic!("unknown authored kind {kind}"),
            }
            .to_owned()
        });
        let n = SourceNode {
            key: (v["node_id"].as_u64().unwrap() as u32, slot.to_owned()),
            parent,
            role,
            language: v
                .get("language")
                .and_then(Value::as_str)
                .unwrap_or(language)
                .to_ascii_lowercase(),
            semantic: if kind == "semantic_container" {
                Some(v["semantic_kind"].as_str().unwrap().to_owned())
            } else {
                None
            },
            span: v.get("span").map(|s| {
                [
                    s["source_id"].as_u64().unwrap() as u32,
                    s["start_byte"].as_u64().unwrap() as u32,
                    s["end_byte"].as_u64().unwrap() as u32,
                ]
            }),
            alternative: if matches!(kind, "inline_math" | "display_math") {
                v.get("speech")
            } else {
                v.get("alt")
            }
            .and_then(Value::as_str)
            .map(str::to_owned),
            cell,
        };
        nodes.push(n.clone());
        n
    }
    fn generated(
        parent: &SourceNode,
        slot: &str,
        role: &str,
        nodes: &mut Vec<SourceNode>,
    ) -> SourceNode {
        let mut n = parent.clone();
        n.key.1 = slot.into();
        n.parent = Some(parent.key.clone());
        n.role = role.into();
        n.semantic = None;
        n.alternative = None;
        n.cell = None;
        nodes.push(n.clone());
        n
    }
    fn walk(values: &[Value], parent: &SourceNode, nodes: &mut Vec<SourceNode>) {
        for v in values {
            let kind = v["kind"].as_str().unwrap();
            if matches!(kind, "anchor" | "soft_break" | "hard_break" | "page_break") {
                continue;
            }
            let n = add(
                v,
                kind,
                Some(parent.key.clone()),
                &parent.language,
                "Source",
                None,
                None,
                nodes,
            );
            match kind {
                "list" | "description_list" => {
                    for item in v["items"].as_array().unwrap() {
                        let li = add(
                            item,
                            if kind == "list" {
                                "list_item"
                            } else {
                                "description_item"
                            },
                            Some(n.key.clone()),
                            &n.language,
                            "Source",
                            None,
                            None,
                            nodes,
                        );
                        if kind == "list" {
                            generated(&li, "ListLabel", "Lbl", nodes);
                        } else {
                            let term = add(
                                &item["term"],
                                "description_term",
                                Some(li.key.clone()),
                                &li.language,
                                "Source",
                                None,
                                None,
                                nodes,
                            );
                            walk(item["term"]["children"].as_array().unwrap(), &term, nodes);
                        }
                        let body = generated(&li, "ListBody", "LBody", nodes);
                        walk(item["blocks"].as_array().unwrap(), &body, nodes);
                    }
                }
                "table" => {
                    if let Some(c) = v.get("caption") {
                        let cap = generated(&n, "Caption", "Caption", nodes);
                        walk(c.as_array().unwrap(), &cap, nodes);
                    }
                    let mut row_index = 0u32;
                    let mut occupied = BTreeSet::new();
                    for (section, slot, role, header) in [
                        ("head", "TableHead", "THead", 1),
                        ("body", "TableBody", "TBody", 0),
                    ] {
                        let group = generated(&n, slot, role, nodes);
                        for row in v[section].as_array().unwrap() {
                            let tr = add(
                                row,
                                "table_row",
                                Some(group.key.clone()),
                                &group.language,
                                "Source",
                                None,
                                None,
                                nodes,
                            );
                            let mut column = 0;
                            for cell in row["cells"].as_array().unwrap() {
                                while occupied.contains(&(row_index, column)) {
                                    column += 1;
                                }
                                let cs = cell["colspan"].as_u64().unwrap() as u32;
                                let rs = cell["rowspan"].as_u64().unwrap() as u32;
                                for r in row_index..row_index + rs {
                                    for c in column..column + cs {
                                        assert!(occupied.insert((r, c)));
                                    }
                                }
                                let td = add(
                                    cell,
                                    "table_cell",
                                    Some(tr.key.clone()),
                                    &tr.language,
                                    "Source",
                                    Some(if header == 1 { "TH" } else { "TD" }),
                                    Some([n.key.0, row_index, column, cs, rs, header]),
                                    nodes,
                                );
                                walk(cell["blocks"].as_array().unwrap(), &td, nodes);
                                column += cs;
                            }
                            row_index += 1;
                        }
                    }
                }
                "figure" | "vector_figure" => {
                    let caption = v["caption"].as_array().unwrap();
                    if !caption.is_empty() {
                        let cap = generated(&n, "Caption", "Caption", nodes);
                        walk(caption, &cap, nodes);
                    }
                }
                "footnote_reference" | "reference" => {
                    let foot = kind == "footnote_reference";
                    let link = generated(
                        &n,
                        if foot {
                            "FootnoteLink"
                        } else {
                            "ReferenceLink"
                        },
                        "Link",
                        nodes,
                    );
                    generated(
                        &link,
                        if foot {
                            "FootnoteLabel"
                        } else {
                            "ReferenceLabel"
                        },
                        if foot { "Lbl" } else { "Span" },
                        nodes,
                    );
                }
                "math_vector_block" => {
                    if let Some(number) = v.get("equation_number").filter(|v| !v.is_null()) {
                        add(
                            number,
                            "equation_number",
                            Some(n.key.clone()),
                            &n.language,
                            "Source",
                            None,
                            None,
                            nodes,
                        );
                    }
                }
                _ => {
                    if let Some(children) = v.get("children").or_else(|| v.get("blocks")) {
                        walk(children.as_array().unwrap(), &n, nodes);
                    }
                }
            }
        }
    }
    let mut nodes = Vec::new();
    let root = add(
        document,
        "document",
        None,
        document["language"].as_str().unwrap(),
        "Source",
        None,
        None,
        &mut nodes,
    );
    walk(document["blocks"].as_array().unwrap(), &root, &mut nodes);
    for note in document["footnotes"].as_array().unwrap() {
        let n = add(
            note,
            "footnote_definition",
            Some(root.key.clone()),
            &root.language,
            "Source",
            None,
            None,
            &mut nodes,
        );
        let link = generated(&n, "FootnoteLink", "Link", &mut nodes);
        generated(&link, "FootnoteLabel", "Lbl", &mut nodes);
        walk(note["blocks"].as_array().unwrap(), &n, &mut nodes);
    }
    nodes
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct MarkedFacts {
    groups: usize,
    semantic: usize,
    artifacts: usize,
    formulas: usize,
    anchors: usize,
    nodes: usize,
    bindings: usize,
    pages: usize,
    cells: usize,
    captions: usize,
}
fn inspect_marked(
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
) -> MarkedFacts {
    let scopes = content.source();
    let display = scopes.display();
    let text = content.text();
    let images = scopes.source();
    assert!(std::ptr::eq(structure.source(), content));
    assert!(std::ptr::eq(text.display(), display));
    inspect_paints(text, images);
    let originals = original_scopes(display);
    assert_eq!(scopes.groups().len(), originals.len());
    let npages = display.source().source().geometry().pages().len();
    assert_eq!(content.pages().len(), npages);
    let mut mcids = vec![0u32; npages];
    let mut cursor = 0;
    let mut text_cursor = 0;
    let mut image_cursor = 0;
    let mut expected_anchors = Vec::new();
    let mut facts = MarkedFacts {
        groups: originals.len(),
        semantic: 0,
        artifacts: 0,
        formulas: 0,
        anchors: 0,
        nodes: structure.nodes().len(),
        bindings: structure.bindings().len(),
        pages: npages,
        cells: structure
            .nodes()
            .iter()
            .filter(|n| n.source().table_cell().is_some())
            .count(),
        captions: structure
            .nodes()
            .iter()
            .filter(|n| n.source().pdf_role() == "Caption")
            .count(),
    };
    for (i, (g, original)) in scopes.groups().iter().zip(&originals).enumerate() {
        assert_eq!(
            (
                g.owner(),
                g.page_index(),
                g.fragment_index(),
                g.role(),
                g.artifact(),
                g.paints()
            ),
            (
                original.owner,
                original.page,
                original.fragment,
                original.role,
                original.artifact,
                original.paints.clone()
            )
        );
        let semantic = original.artifact.is_none();
        let mcid = semantic.then(|| {
            let n = mcids[original.page as usize];
            mcids[original.page as usize] += 1;
            n
        });
        assert_eq!(g.mcid(), mcid);
        facts.semantic += usize::from(semantic);
        facts.artifacts += usize::from(!semantic);
        facts.formulas += usize::from(semantic && g.role() == Role::Formula);
        let actual = if semantic {
            original.actual.as_deref()
        } else {
            None
        };
        let alt = if semantic {
            original.alternative.as_deref()
        } else {
            None
        };
        assert_eq!(scopes.actual_text(i), actual);
        assert_eq!(scopes.alternative(i), alt);
        let begin = match original.artifact {
            Some(Artifact::RepeatedHeader) => {
                "/Artifact << /Type /Pagination /Subtype /Header >> BDC\n".to_owned()
            }
            Some(Artifact::FootnoteSeparator) => "/Artifact << /Type /Layout >> BDC\n".to_owned(),
            Some(_) => panic!("unexpected Column running region"),
            None => {
                let role = match original.role {
                    Role::Text | Role::EquationNumber => "Span",
                    Role::Label => "Lbl",
                    Role::Formula => "Formula",
                    Role::Figure => "Figure",
                    Role::Separator => panic!("semantic separator"),
                };
                format!(
                    "/{role} << /MCID {} >> BDC\n{}",
                    mcid.unwrap(),
                    actual
                        .map(|a| format!("/Span << /ActualText {} >> BDC\n", replacement_token(a)))
                        .unwrap_or_default()
                )
            }
        };
        assert_eq!(scopes.begin_bytes(i).unwrap(), begin.as_bytes());
        let closing = if actual.is_some() {
            b"EMC\nEMC\n".as_slice()
        } else {
            b"EMC\n".as_slice()
        };
        assert_eq!(scopes.end_bytes(i).unwrap(), closing);
        let inline = semantic
            && matches!(
                original.role,
                Role::Text | Role::Label | Role::EquationNumber
            );
        let mut expected = b"q\n".to_vec();
        expected.extend_from_slice(begin.as_bytes());
        if inline {
            assert!(actual.is_none());
            expected.extend_from_slice(
                format!(
                    "/Span << /ActualText {} >> BDC\n",
                    replacement_token(&original.whole)
                )
                .as_bytes(),
            );
        }
        let anchor = if actual.is_some() {
            assert_eq!(original.paints.len(), 1);
            match display.paints()[original.paints.start] {
                Paint::Math(i) => {
                    let d = &display.math().draws()[i];
                    match d.paint() {
                        BookV2MathPaint::Vector(v) => Some((v.viewport(), d.terminal().baseline())),
                        BookV2MathPaint::Native(_) => None,
                    }
                }
                Paint::Image(i) => {
                    let d = &display.images().draws()[i];
                    Some((
                        d.paint().viewport(),
                        d.fragment().fragment().baseline().unwrap(),
                    ))
                }
                _ => None,
            }
        } else {
            None
        };
        if let Some((v, baseline)) = anchor {
            expected_anchors.push((original.page, i, original.paints.start, v, baseline));
            let remaining = &content.group_bytes(i).unwrap()[expected.len()..];
            let end = remaining.windows(3).position(|b| b == b"ET\n").unwrap() + 3;
            let bytes = &remaining[..end];
            let t = std::str::from_utf8(bytes)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>();
            assert_eq!(
                &t[..16],
                [
                    "BT", "/BMA", "1", "Tf", "3", "Tr", "0", "Tc", "0", "Tw", "100", "Tz", "0",
                    "TL", "0", "Ts"
                ]
            );
            for (token, raw) in t[16..22].iter().zip([
                v.width().get().raw(),
                0,
                0,
                -v.height().get().raw(),
                v.x().raw(),
                baseline.raw(),
            ]) {
                assert_eq!(fixed(token), i128::from(raw));
            }
            assert_eq!(&t[22..], ["Tm", "<00>", "Tj", "ET"]);
            expected.extend_from_slice(bytes);
        }
        for paint in original.paints.clone() {
            for c in text.for_paint(paint).unwrap() {
                assert!(std::ptr::eq(c, &text.commands()[text_cursor]));
                expected.extend_from_slice(text.command_bytes(text_cursor).unwrap());
                text_cursor += 1;
            }
            if images
                .commands()
                .get(image_cursor)
                .is_some_and(|c| c.paint_index() == paint)
            {
                let c = &images.commands()[image_cursor];
                assert_eq!(Some(c.owner()), original.owner);
                expected.extend_from_slice(images.command_bytes(image_cursor).unwrap());
                expected.push(b'\n');
                image_cursor += 1;
            }
            assert!(std::ptr::eq(scopes.for_paint(paint).unwrap(), g));
            cursor += 1;
        }
        if inline {
            expected.extend_from_slice(b"EMC\n");
        }
        expected.extend_from_slice(closing);
        expected.extend_from_slice(b"Q\n");
        assert_eq!(content.group_bytes(i).unwrap(), expected);
        let encoded = std::str::from_utf8(content.group_bytes(i).unwrap()).unwrap();
        assert_eq!(encoded.matches("/MCID ").count(), usize::from(semantic));
        assert_eq!(
            encoded.matches("/ActualText ").count(),
            usize::from(inline || actual.is_some())
        );
    }
    assert_eq!(
        (cursor, text_cursor, image_cursor),
        (
            display.paints().len(),
            text.commands().len(),
            images.commands().len()
        )
    );
    assert_eq!(content.anchors().len(), expected_anchors.len());
    for (a, (page, group, paint, viewport, baseline)) in
        content.anchors().iter().zip(&expected_anchors)
    {
        assert_eq!(
            (
                a.page_index(),
                a.group_index(),
                a.paint_index(),
                a.viewport(),
                a.baseline()
            ),
            (*page, *group, *paint, *viewport, *baseline)
        );
    }
    facts.anchors = expected_anchors.len();
    let mut group_cursor = 0;
    let mut bytes = 0;
    for (page, p) in content.pages().iter().enumerate() {
        assert_eq!(p.page_index(), page as u32);
        assert_eq!(p.groups().start, group_cursor);
        let mut expected = b"q\n".to_vec();
        for g in p.groups() {
            assert_eq!(scopes.groups()[g].page_index(), page as u32);
            expected.extend_from_slice(content.group_bytes(g).unwrap());
            group_cursor += 1;
        }
        expected.extend_from_slice(b"Q\n");
        assert_eq!(content.page_bytes(page).unwrap(), expected);
        bytes += expected.len();
    }
    assert_eq!(
        (group_cursor, bytes),
        (scopes.groups().len(), content.byte_length())
    );
    let first = text
        .source()
        .next_object()
        .max(images.source().next_object());
    assert_eq!(
        content.anchor_objects().len(),
        if expected_anchors.is_empty() { 0 } else { 3 }
    );
    let mut cursor = 0;
    for (i, o) in content.anchor_objects().iter().enumerate() {
        assert_eq!(o.id().get(), first + i as u32);
        assert_eq!(
            o.role(),
            [
                BookV2SemanticAnchorRole::Font,
                BookV2SemanticAnchorRole::Glyph,
                BookV2SemanticAnchorRole::ToUnicode
            ][i]
        );
        assert_eq!(o.byte_range().start, cursor);
        cursor = o.byte_range().end;
        let b = &content.anchor_object_bytes()[o.byte_range()];
        assert!(b.starts_with(format!("{} 0 obj\n", o.id().get()).as_bytes()));
        if i == 0 {
            let s = std::str::from_utf8(b).unwrap();
            for fact in [
                "/Subtype /Type3",
                "/FontBBox [0 0 1000 1000]",
                "/FontMatrix [0.001 0 0 0.001 0 0]",
                "/Widths [1000]",
                "/FirstChar 0 /LastChar 0",
                "/Differences [0 /anchor]",
            ] {
                assert!(s.contains(fact));
            }
        } else if i == 1 {
            assert!(b
                .windows(b"1000 0 0 0 1000 1000 d1\n".len())
                .any(|w| w == b"1000 0 0 0 1000 1000 d1\n"));
        } else {
            assert!(b.windows(b"<00> <FFFC>".len()).any(|w| w == b"<00> <FFFC>"));
        }
    }
    assert_eq!(cursor, content.anchor_object_bytes().len());
    assert_eq!(
        content.next_object(),
        first + content.anchor_objects().len() as u32
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
    let nodes = original_source_nodes(&wire["document"]);
    assert_eq!(nodes.len(), structure.nodes().len());
    let lookup = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.key.clone(), i))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(lookup.len(), nodes.len());
    let mut children = vec![Vec::new(); nodes.len()];
    for (i, (n, e)) in structure.nodes().iter().zip(&nodes).enumerate() {
        let s = n.source();
        assert_eq!(
            (s.key().owner().get(), format!("{:?}", s.key().slot())),
            e.key
        );
        assert_eq!(s.pdf_role(), e.role);
        assert_eq!(s.language().to_ascii_lowercase(), e.language);
        assert_eq!(s.semantic_kind(), e.semantic.as_deref());
        assert_eq!(s.alternative(), e.alternative.as_deref());
        assert_eq!(
            s.source_span().map(|s| [
                s.source_id().get(),
                s.start_byte().get(),
                s.end_byte().get()
            ]),
            e.span
        );
        assert_eq!(
            s.table_cell().map(|c| [
                c.table.get(),
                c.row,
                c.column,
                u32::from(c.colspan),
                u32::from(c.rowspan),
                u32::from(c.header)
            ]),
            e.cell
        );
        let parent = e.parent.as_ref().map(|k| lookup[k]);
        assert_eq!(n.parent(), parent);
        assert_eq!(
            n.depth(),
            parent.map_or(1, |p| structure.nodes()[p].depth() + 1)
        );
        if let Some(p) = parent {
            assert!(p < i);
            children[p].push(i);
        }
        assert_eq!(structure.node_index(s.key()), Some(i));
    }
    for (i, expected) in children.iter().enumerate() {
        let mut actual = Vec::new();
        let mut next = structure.nodes()[i].first_child();
        while let Some(c) = next {
            assert!(!actual.contains(&c));
            actual.push(c);
            next = structure.nodes()[c].next_sibling();
        }
        assert_eq!(&actual, expected);
    }
    let mut expected_binding = 0;
    let mut per_node = vec![Vec::new(); nodes.len()];
    for (i, g) in scopes.groups().iter().enumerate() {
        if g.artifact().is_some() {
            assert!(structure.for_group(i).is_none());
            continue;
        }
        let owner = g.owner().unwrap().get();
        let mut key = (owner, "Source".to_owned());
        let original = &nodes[lookup[&key]];
        if g.role() == Role::Label {
            key.1 = if original.role == "LI" {
                "ListLabel"
            } else {
                "FootnoteLabel"
            }
            .into();
        } else if g.role() == Role::Text && original.role == "Reference" {
            key.1 = if lookup.contains_key(&(owner, "FootnoteLabel".into())) {
                "FootnoteLabel"
            } else {
                "ReferenceLabel"
            }
            .into();
        }
        let node = lookup[&key];
        let b = structure.for_group(i).unwrap();
        assert!(std::ptr::eq(b, &structure.bindings()[expected_binding]));
        assert_eq!(
            (b.node_index(), b.group_index(), b.page_index(), b.mcid()),
            (node, i, g.page_index(), g.mcid().unwrap())
        );
        per_node[node].push(expected_binding);
        expected_binding += 1;
    }
    assert_eq!(expected_binding, structure.bindings().len());
    for (i, expected) in per_node.iter().enumerate() {
        let mut actual = Vec::new();
        let mut next = structure.nodes()[i].first_binding();
        while let Some(b) = next {
            assert!(!actual.contains(&b));
            actual.push(b);
            next = structure.bindings()[b].next_for_node();
        }
        assert_eq!(&actual, expected);
    }
    assert!(scopes.for_paint(usize::MAX).is_none());
    assert!(content.page_bytes(usize::MAX).is_none());
    assert!(structure.for_group(usize::MAX).is_none());
    facts
}

use crate::book_v2_resources::with_budgeted_book_v2_column_marked as marked_drive;
fn input_marked(
    case: &str,
    input: &PreparedBookV2Resources,
    caps: &M4EffectiveResourceLimits,
) -> MarkedFacts {
    let mut budget = BookV2ColumnPageBudget::new(caps, MAXIMUM);
    marked_drive(input, caps, MODE, &mut budget, 4, |bundle, _, observed| {
        let facts = inspect_marked(bundle.content(), bundle.structure());
        assert!(std::ptr::eq(bundle.display(), bundle.paints().display()));
        assert_eq!(
            bundle.structure().output_charge(),
            bundle.content().output_charge()
        );
        assert_eq!(observed.output_charge(), bundle.content().output_charge());
        assert_eq!(
            bundle.constructor_charges()[10][2],
            bundle.content().output_charge()
        );
        marked_probes::emit(
            case,
            bundle.content(),
            bundle.structure(),
            caps,
            Some(bundle.encoded_prefix()),
            Some(observed),
            "marked_driver",
            MAXIMUM,
            bundle.constructor_charges(),
            bundle.paints().reservation_sizes(),
            Some(bundle.caller_before()),
        );
        facts
    })
    .unwrap_or_else(|e| panic!("{case}: {e:?}"))
}

#[test]
fn book_v2_column_marked_body_notes_balanced_named_and_global_fragment_boundaries() {
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
        let facts = input_marked(&format!("marked-list-notes-{balanced}"), &input, &caps);
        assert!(facts.semantic > 0 && facts.artifacts > 0 && facts.bindings == facts.semantic);
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
            let facts = input_marked(&format!("marked-named-{blanks}-{balanced}"), &input, &caps);
            assert!(facts.pages > blanks && facts.semantic > 0);
        }
    }
}

#[test]
fn book_v2_column_marked_original_vector_native_numbers_png_jpeg_svg_and_formula_anchors() {
    let caps = display_caps();
    for kind in ["vector", "native", "png", "jpeg", "svg"] {
        for balanced in [false, true] {
            let mut data=super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
            anchors(&mut data);
            balance(&mut data, balanced);
            let root = Root::new();
            let input = match kind {
                "vector" => vector_input(&root, data, &caps),
                "native" => {
                    let source = data["text_buffers"][0]["utf8"]
                        .as_str()
                        .unwrap()
                        .as_bytes()
                        .to_vec();
                    native_input_with_source(&root, data, &source, &caps)
                }
                _ => figure_input(
                    &root,
                    data,
                    if kind == "png" {
                        PNG
                    } else if kind == "jpeg" {
                        JPEG
                    } else {
                        SVG
                    },
                    &caps,
                ),
            };
            let facts = input_marked(&format!("marked-media-{kind}-{balanced}"), &input, &caps);
            assert_eq!(facts.formulas > 0, matches!(kind, "native" | "vector"));
            assert_eq!(facts.anchors > 0, kind == "vector");
            assert!(facts.semantic > 0 && facts.bindings == facts.semantic);
        }
    }
}

#[test]
fn book_v2_column_marked_actual_nested_headers_are_artifacts_without_source_bindings() {
    let caps = display_caps();
    for vector in [false, true] {
        for notes in [false, true] {
            let mut data =
                super::super::super::super::math_terminals::header_data(vector, notes, true);
            anchors(&mut data);
            balance(&mut data, !notes);
            let root = Root::new();
            let input = if vector {
                vector_input(&root, data, &caps)
            } else {
                let source = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                native_input_with_source(&root, data, &source, &caps)
            };
            let facts = input_marked(&format!("marked-header-{vector}-{notes}"), &input, &caps);
            assert!(facts.artifacts > 0 && facts.formulas > 0);
            assert!(facts.groups > facts.bindings);
        }
    }
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let facts = input_marked("marked-balanced-six-stream-notes", &input, &caps);
    assert!(facts.formulas > 0 && facts.anchors > 0 && facts.artifacts > 0);
}

#[test]
fn book_v2_column_marked_fraction_rules_and_ambiguous_text_have_one_parent_replacement() {
    let caps = display_caps();
    for repeated in [false, true] {
        let data = fraction_data(repeated);
        let text = data["text_buffers"][0]["utf8"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        let root = Root::new();
        let input = native_input_with_source(&root, data, &text, &caps);
        let facts = input_marked(&format!("marked-fraction-{repeated}"), &input, &caps);
        assert!(facts.formulas > 0);
        assert_eq!(facts.anchors, 0);
        assert_eq!(facts.artifacts > 0, repeated);
    }
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"
    ));
    let root = Root::new();
    let input = display_input(
        &root,
        plain_data("ABA fi X D E f i", false, false),
        Some(FONT),
        &caps,
    );
    let facts = input_marked("marked-real-gsub-extraction", &input, &caps);
    assert!(facts.semantic > 0 && facts.formulas == 0);
}

#[test]
fn book_v2_column_marked_alias_figures_keep_distinct_mcid_owner_scopes() {
    let caps = display_caps();
    for (kind, bytes) in [("png", PNG), ("jpeg", JPEG), ("svg", PAINT_CURVES)] {
        let mut data=super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
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
        marked_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let selected = bundle.paints().rasters().source();
                assert_eq!(selected.images().len(), 1);
                let ids = selected
                    .uses()
                    .iter()
                    .map(|u| u.usage().image().image_id().get())
                    .collect::<BTreeSet<_>>();
                assert_eq!(ids, BTreeSet::from([0, 1]));
                let scopes = bundle.content().source();
                for u in selected.uses() {
                    let scope = scopes.for_paint(u.paint_index()).unwrap();
                    assert_eq!(scope.paints(), u.paint_index()..u.paint_index() + 1);
                    assert_eq!(scope.owner(), Some(u.usage().owner()));
                    assert!(scope.mcid().is_some());
                    assert_eq!(scope.role(), Role::Figure);
                }
                inspect_marked(bundle.content(), bundle.structure());
                marked_probes::emit(
                    &format!("marked-alias-curves-{kind}"),
                    bundle.content(),
                    bundle.structure(),
                    &caps,
                    Some(bundle.encoded_prefix()),
                    Some(observed),
                    "marked_driver",
                    MAXIMUM,
                    bundle.constructor_charges(),
                    bundle.paints().reservation_sizes(),
                    Some(bundle.caller_before()),
                );
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_marked_empty_forced_zero_and_unreferenced_source_skeletons_remain_original() {
    let caps = display_caps();
    for table in [false, true] {
        for balanced in [false, true] {
            let mut data = if table {
                super::super::super::super::super::super::data(false, "common", "Pro")
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
            renumber(&mut data);
            let root = Root::new();
            let input = display_input(&root, data, None, &caps);
            let facts = input_marked(&format!("marked-zero-{table}-{balanced}"), &input, &caps);
            assert_eq!((facts.groups, facts.bindings, facts.anchors), (0, 0, 0));
            assert!(facts.nodes > 0 && facts.pages > 0);
        }
    }
    let mut data=super::super::super::super::super::super::super::super::super::block_widths::column_block_data("vector",true,false);
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
    marked_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            assert_eq!(
                bundle
                    .display()
                    .source()
                    .source()
                    .unreferenced_definitions(),
                1
            );
            let structure = bundle.structure();
            assert!(structure
                .nodes()
                .iter()
                .any(|n| excluded.contains(&n.source().key().owner().get())));
            for n in structure.nodes() {
                if excluded.contains(&n.source().key().owner().get()) {
                    assert!(n.first_binding().is_none());
                }
            }
            assert!(bundle
                .content()
                .source()
                .groups()
                .iter()
                .all(|g| g.owner().map_or(true, |o| !excluded.contains(&o.get()))));
            inspect_marked(bundle.content(), structure);
            marked_probes::emit(
                "marked-unused-rich",
                bundle.content(),
                structure,
                &caps,
                Some(bundle.encoded_prefix()),
                Some(observed),
                "marked_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.paints().reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_marked_rowspan_nested_caption_and_forced_table_hierarchy_is_authored() {
    let caps = display_caps();
    for (mode, height, balanced) in [
        ("span", 256, false),
        ("nested-body-span", 544, false),
        ("nested-body-caption", 512, true),
        ("forced", 256, false),
    ] {
        let mut data = super::super::super::super::super::super::data(
            false,
            mode,
            "Pro Pro Pro Pro Pro Pro Pro",
        );
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["body"]["height"] = (height * 65536).into();
        }
        anchors(&mut data);
        balance(&mut data, balanced);
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = input_marked(&format!("marked-table-{mode}-{balanced}"), &input, &caps);
        assert!(facts.semantic > 0 && facts.cells > 0);
        if mode == "nested-body-caption" {
            assert!(facts.captions > 0);
        }
    }
}

fn semantic_columns(data: &mut Value, height: i64) {
    for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
        m["width"] = 12_000_000.into();
        m["height"] = 12_000_000.into();
        m["trim"] = json!({"x":0,"y":0,"width":12_000_000,"height":12_000_000});
        m["body"] = json!({"x":500000,"y":500000,"width":8100000,"height":height});
        m["column_layout"] = json!({"count":2,"gap":100000,"fill":"sequential","balance":"none"});
    }
}
fn semantic_renumber(v: &mut Value, next: &mut u32) {
    match v {
        Value::Array(a) => {
            for v in a {
                semantic_renumber(v, next)
            }
        }
        Value::Object(o) => {
            if let Some(n) = o.get_mut("node_id") {
                *n = (*next).into();
                *next += 1;
            }
            for k in [
                "term",
                "blocks",
                "children",
                "items",
                "caption",
                "head",
                "body",
                "cells",
                "equation_number",
                "footnotes",
            ] {
                if let Some(v) = o.get_mut(k) {
                    semantic_renumber(v, next);
                }
            }
        }
        _ => (),
    }
}
fn description_data() -> Value {
    let term = "Result ".repeat(12);
    let text = format!("{term}Proof");
    let mut data = source_data(&text);
    let span = |start, end| json!({"source_id":0,"start_byte":start,"end_byte":end});
    let child = |start, end| json!({"kind":"text","node_id":0,"span":span(start,end),"text_span":{"text_id":0,"start_byte":start,"end_byte":end}});
    data["document"]["blocks"] = json!([{"kind":"description_list","node_id":0,"classes":[],"span":span(0,text.len()),"language":"en-us","items":[{"node_id":0,"span":span(0,text.len()),"language":"de-de","term":{"node_id":0,"span":span(0,term.len()),"classes":[],"language":"fr-ca","children":[{"kind":"link","node_id":0,"span":span(0,term.len()),"target":{"kind":"internal","anchor_id":"definition"},"children":[child(0,term.len())]}]},"blocks":[{"kind":"paragraph","node_id":0,"classes":[],"span":span(term.len(),text.len()),"children":[{"kind":"anchor","node_id":0,"span":span(term.len(),term.len()),"anchor_id":"definition"},child(term.len(),text.len())]}]}]}]);
    let length = |name, value| json!({"name":name,"important":false,"value":{"kind":"length","value":value}});
    data["style_sheet"]["rules"].as_array_mut().unwrap().extend([json!({"style_id":"description","selector":"description_list","source_order":3,"extends":"paragraph-text","declarations":[length("start_indent",131072),length("end_indent",65536),length("space_before",65536),length("space_after",65536)]}),json!({"style_id":"term","selector":"description_term","source_order":4,"extends":null,"declarations":[length("start_indent",65536)]})]);
    semantic_columns(&mut data, 3000000);
    semantic_renumber(&mut data["document"], &mut 0);
    data
}
fn number_reference_data() -> Value {
    let text = "Result 1.12";
    let mut data = source_data(text);
    let group = data["document"]["blocks"][0].clone();
    let reference = |at| json!({"kind":"paragraph","node_id":0,"classes":[],"span":{"source_id":0,"start_byte":at,"end_byte":at},"children":[{"kind":"reference","node_id":0,"target":"result.number","format":"number","span":{"source_id":0,"start_byte":at,"end_byte":at}}]});
    data["document"]["blocks"] = json!([reference(0), group, reference(text.len())]);
    semantic_columns(&mut data, 3000000);
    renumber(&mut data);
    let group = &data["document"]["blocks"][1];
    data["document"]["number_bindings"] = json!([{"anchor_id":"result.number","owner_node_id":group["node_id"],"label_node_id":group["blocks"][0]["children"][0]["node_id"],"text_span":{"text_id":0,"start_byte":7,"end_byte":text.len()}}]);
    data
}
#[test]
fn book_v2_column_marked_description_languages_links_and_generated_number_source_slots() {
    let caps = display_caps();
    for (case, data) in [
        ("marked-description-languages", description_data()),
        ("marked-number-references", number_reference_data()),
    ] {
        let root = Root::new();
        let input = display_input(&root, data, None, &caps);
        let facts = input_marked(case, &input, &caps);
        assert!(facts.semantic > 0 && facts.nodes > 0);
    }
}

#[test]
fn book_v2_column_marked_all_three_stages_obey_exact_one_short_and_inherited_four_caps() {
    let caps = display_caps();
    for kind in ["vector", "native", "png"] {
        let data=super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,false,false);
        let root = Root::new();
        let input = match kind {
            "vector" => vector_input(&root, data, &caps),
            "native" => {
                let source = data["text_buffers"][0]["utf8"]
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .to_vec();
                native_input_with_source(&root, data, &source, &caps)
            }
            _ => figure_input(&root, data, PNG, &caps),
        };
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        marked_drive(&input,&caps,MODE,&mut budget,4,|bundle,_,_|{
    macro_rules! stage {($builder:ident,$source:expr $(,$text:expr)?)=>{{let source=$source;let base=[source.record_charge(),source.spool_charge(),source.output_charge(),source.work_steps()];let run=|prior:[u64;4],maximum:u64,limits:&M4EffectiveResourceLimits|{let(mut r,mut s,mut o,mut w)=(0,0,0,0);let made=$builder::new_counted(source,$($text,)?limits,maximum,prior[0],prior[1],prior[2],prior[3],&mut r,&mut s,&mut o,&mut w);let constructor=[r,s,o,w];let mut b=match made{Ok(b)=>b,Err(e)=>return EncodedAttempt{constructor,after:constructor,fingerprint:None,error:Some(format!("{e:?}"))}};let result=b.build();let fingerprint=result.as_ref().ok().map(|v|v.fingerprint());let error=result.err().map(|e|format!("{e:?}"));EncodedAttempt{constructor,after:[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()],fingerprint,error}};let baseline=run([0;4],MAXIMUM,&caps);assert!(baseline.error.is_none(),"{kind} {}: {baseline:?}",stringify!($builder));for(axis,cap,error)in[(0,caps.base().get().max_fragments,"Records"),(1,caps.base().get().max_spool_bytes,"Spool"),(2,caps.base().get().max_output_bytes,"Output")]{let delta=baseline.after[axis] as i128-base[axis] as i128;if delta>0{let mut prior=[0;4];prior[axis]=cap-delta as u64;let exact=run(prior,MAXIMUM,&caps);assert!(exact.error.is_none(),"{} axis{axis}: {exact:?}",stringify!($builder));assert_eq!(exact.after[axis],cap);assert_eq!(exact.fingerprint,baseline.fingerprint);prior[axis]+=1;let short=run(prior,MAXIMUM,&caps);assert!(short.error.as_deref().is_some_and(|e|e.contains(error)),"{} axis{axis}: {short:?}",stringify!($builder));assert!(short.after[axis]<=cap);}else{let mut prior=[0;4];prior[axis]=cap;let exact=run(prior,MAXIMUM,&caps);assert!(exact.error.is_none());assert_eq!(exact.after[axis] as i128,cap as i128+delta);prior[axis]+=1;let overage=run(prior,MAXIMUM,&caps);assert!(overage.error.as_deref().is_some_and(|e|e.contains(error)));assert_eq!(overage.constructor[axis],cap+1);}}
        let exact=run([0;4],baseline.after[3],&caps);assert!(exact.error.is_none());assert_eq!(exact.fingerprint,baseline.fingerprint);let short=run([0;4],baseline.after[3]-1,&caps);assert!(short.error.as_deref().is_some_and(|e|e.contains("Work")));assert!(short.after[3]<=baseline.after[3]-1);let changed=changed_caps(&caps,|c|c.max_fragments+=1);let foreign=run([0;4],MAXIMUM,&changed);assert!(foreign.error.as_deref().is_some_and(|e|e.contains("Identity")));assert_eq!(foreign.after,base);
        let cap=caps.base().get().max_fragments;let ctor=run([cap-1,0,0,0],MAXIMUM,&caps);assert_eq!(ctor.constructor[0],cap);let short=run([cap,0,0,0],MAXIMUM,&caps);assert_eq!(short.constructor[0],cap);assert!(short.error.as_deref().is_some_and(|e|e.contains("Records")));let(mut r,mut s,mut o,mut w)=(0,0,0,0);let exact_ctor=$builder::new_counted(source,$($text,)?&caps,base[3],0,0,0,0,&mut r,&mut s,&mut o,&mut w);assert!(exact_ctor.is_ok());drop(exact_ctor);let one_short=$builder::new_counted(source,$($text,)?&caps,base[3]-1,0,0,0,0,&mut r,&mut s,&mut o,&mut w);assert!(matches!(one_short,Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)));assert_eq!([r,s,o,w],[base[0]+1,base[1],base[2],base[3]]);
    }};}
    let content=bundle.content();let scopes=content.source();stage!(BookV2ColumnMarkedScopeBuilder,scopes.source());stage!(BookV2ColumnMarkedContentBuilder,scopes,content.text());stage!(BookV2ColumnSourceStructureBuilder,content);
}).unwrap();
    }
}

#[test]
fn book_v2_column_marked_driver_retains_scopes_and_signed_content_output_on_every_failure_phase() {
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf"
    ));
    let caps = display_caps();
    for negative in [false, true] {
        let negative_text = "AB ".repeat(48);
        let mut data = plain_data(if negative { &negative_text } else { "Pro" }, false, false);
        if negative {
            for m in data["page_masters"]["masters"].as_array_mut().unwrap() {
                m["width"] = (660 * 65536).into();
                m["trim"]["width"] = (660 * 65536).into();
                m["body"]["width"] = (610 * 65536).into();
            }
        }
        let root = Root::new();
        let input = display_input(&root, data.clone(), negative.then_some(FONT), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        let mut phases = None;
        marked_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |bundle, _, observed| {
                let scopes = bundle.content().source();
                let content = bundle.content();
                let tree = bundle.structure();
                let p = (
                    [
                        scopes.record_charge(),
                        scopes.spool_charge(),
                        scopes.output_charge(),
                        scopes.work_steps(),
                    ],
                    [
                        content.record_charge(),
                        content.spool_charge(),
                        content.output_charge(),
                        content.work_steps(),
                    ],
                    [
                        tree.record_charge(),
                        tree.spool_charge(),
                        tree.output_charge(),
                        tree.work_steps(),
                    ],
                );
                assert!(p.0[2] > bundle.encoded_prefix().output_charge());
                if negative {
                    assert!(
                        p.1[2] < p.0[2],
                        "actual ambiguous source must settle lower: {p:?}"
                    );
                } else {
                    assert!(
                        p.1[2] > p.0[2],
                        "ordinary source must add materialized output: {p:?}"
                    );
                }
                phases = Some(p);
                inspect_marked(content, tree);
                marked_probes::emit(
                    &format!("marked-signed-settlement-{negative}"),
                    content,
                    tree,
                    &caps,
                    Some(bundle.encoded_prefix()),
                    Some(observed),
                    "marked_driver",
                    MAXIMUM,
                    bundle.constructor_charges(),
                    bundle.paints().reservation_sizes(),
                    Some(bundle.caller_before()),
                );
            },
        )
        .unwrap();
        let (scopes, content, structure) = phases.unwrap();
        for (label, stage, maximum, expected_output, expected_records, expected_spool) in [
            (
                "scopes-post-reservation",
                "column marked scopes",
                scopes[3] - 1,
                scopes[2],
                scopes[0],
                scopes[1],
            ),
            (
                "content-before-reservation",
                "column marked content",
                scopes[3],
                scopes[2],
                scopes[0] + 1,
                scopes[1],
            ),
            (
                "content-post-reservation",
                "column marked content",
                content[3] - 1,
                content[2],
                content[0],
                content[1],
            ),
            (
                "structure-post-reservation",
                "column source structure",
                structure[3] - 1,
                content[2],
                structure[0],
                structure[1],
            ),
        ] {
            let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
            let mut delivered = false;
            let result = marked_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
                delivered = true
            });
            assert!(result.is_err(), "{label} negative{negative}");
            let failure = format!("{result:?}");
            assert!(
                failure.contains(stage) && failure.contains("Work"),
                "{label}: {failure}"
            );
            assert!(!delivered);
            assert_eq!(
                budget.observation().record_charge(),
                expected_records,
                "{label}"
            );
            assert_eq!(
                budget.observation().spool_charge(),
                expected_spool,
                "{label}"
            );
            assert_eq!(
                budget.observation().output_charge(),
                expected_output,
                "{label} negative{negative}: {result:?}"
            );
            assert!(budget.observation().work_steps() <= maximum);
            let prefix = paint_observation(budget.observation());
            assert!(
                marked_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| delivered =
                    true)
                .is_err()
            );
            assert!(!delivered);
            assert!(
                budget.observation().record_charge() >= prefix[0]
                    && budget.observation().spool_charge() >= prefix[1]
                    && budget.observation().work_steps() >= prefix[3]
            );
            assert_eq!(budget.observation().output_charge(), prefix[2]);
        }
        // The completed scope reservation is retained even when the next
        // constructor cannot reserve its own record.
        let limits = changed_caps(&caps, |c| c.max_fragments = scopes[0]);
        let root = Root::new();
        let input = display_input(&root, data, negative.then_some(FONT), &limits);
        let mut budget = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        let mut delivered = false;
        let result = marked_drive(&input, &limits, MODE, &mut budget, 4, |_, _, _| {
            delivered = true
        });
        assert!(result.is_err());
        let failure = format!("{result:?}");
        assert!(
            failure.contains("column marked content constructor") && failure.contains("Records"),
            "{failure}"
        );
        assert!(!delivered);
        assert_eq!(budget.observation().record_charge(), scopes[0]);
        assert_eq!(budget.observation().output_charge(), scopes[2]);
    }
}

#[test]
fn book_v2_column_marked_repeated_stages_charge_full_output_after_consuming_merge_credit_once() {
    let caps = display_caps();
    let root = Root::new();
    let input = vector_input(&root, balanced_six_stream_data(), &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    marked_drive(&input, &caps, MODE, &mut budget, 4, |bundle, _, _| {
        let content = bundle.content();
        let scopes = content.source();
        let mut sb =
            BookV2ColumnMarkedScopeBuilder::new(scopes.source(), &caps, MAXIMUM, 0, 0, 0, 0)
                .unwrap();
        let a = sb.build().unwrap();
        let ac = [
            a.record_charge(),
            a.spool_charge(),
            a.output_charge(),
            a.work_steps(),
        ];
        let fp = a.fingerprint();
        let again = sb.build().unwrap();
        assert_eq!(again.fingerprint(), fp);
        let delimiter = (0..a.groups().len())
            .map(|i| a.begin_bytes(i).unwrap().len() + a.end_bytes(i).unwrap().len())
            .sum::<usize>() as u64;
        assert_eq!(again.output_charge() - ac[2], delimiter);
        assert!(
            again.record_charge() > ac[0]
                && again.spool_charge() > ac[1]
                && again.work_steps() > ac[3]
        );
        let mut cb = BookV2ColumnMarkedContentBuilder::new(
            scopes,
            content.text(),
            &caps,
            MAXIMUM,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        let first = cb.build().unwrap();
        let materialized = first.byte_length() + first.anchor_object_bytes().len();
        let credit = content.text().byte_length()
            + scopes.source().bytes().len()
            + delimiter as usize
            + (0..content.text().commands().len())
                .map(|i| content.text().actual_text(i).map_or(0, |b| b.len()))
                .sum::<usize>();
        assert_eq!(
            first.output_charge() as i128 - scopes.output_charge() as i128,
            materialized as i128 - credit as i128
        );
        let initial = [
            first.record_charge(),
            first.spool_charge(),
            first.output_charge(),
            first.work_steps(),
        ];
        let fp = first.fingerprint();
        let second = cb.build().unwrap();
        assert_eq!(second.fingerprint(), fp);
        assert_eq!(second.output_charge() - initial[2], materialized as u64);
        assert!(
            second.record_charge() > initial[0]
                && second.spool_charge() > initial[1]
                && second.work_steps() > initial[3]
        );
        let mut st =
            BookV2ColumnSourceStructureBuilder::new(content, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
        let a = st.build().unwrap();
        let b = st.build().unwrap();
        assert_eq!(a.fingerprint(), b.fingerprint());
        assert_eq!(b.output_charge(), content.output_charge());
        assert!(
            b.record_charge() > a.record_charge()
                && b.spool_charge() > a.spool_charge()
                && b.work_steps() > a.work_steps()
        );
        for (maximum, expected_output) in [(first.work_steps() - 1, first.output_charge())] {
            let mut b = BookV2ColumnMarkedContentBuilder::new(
                scopes,
                content.text(),
                &caps,
                maximum,
                0,
                0,
                0,
                0,
            )
            .unwrap();
            assert!(matches!(
                b.build(),
                Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)
            ));
            assert_eq!(b.output_charge(), expected_output);
            let prefix = [
                b.record_charge(),
                b.spool_charge(),
                b.output_charge(),
                b.work_steps(),
            ];
            assert!(b.build().is_err());
            assert_eq!(
                [
                    b.record_charge(),
                    b.spool_charge(),
                    b.output_charge(),
                    b.work_steps()
                ],
                prefix
            );
        }
    })
    .unwrap();
}

#[test]
fn book_v2_column_marked_driver_exact_peak_limits_callback_failure_and_cumulative_retry() {
    let caps = display_caps();
    let data = plain_data("Pro", true, false);
    let root = Root::new();
    let input = display_input(&root, data.clone(), None, &caps);
    let mut warm = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut peak = 0;
    marked_drive(&input, &caps, MODE, &mut warm, 4, |bundle, _, _| {
        peak = bundle
            .encoded_prefix()
            .output_charge()
            .max(bundle.content().source().output_charge())
            .max(bundle.content().output_charge());
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
            let result = marked_drive(
                &input,
                &limits,
                MODE,
                &mut budget,
                4,
                |bundle, _, observed| {
                    delivered = true;
                    if axis == 2 {
                        let actual_peak = bundle
                            .encoded_prefix()
                            .output_charge()
                            .max(bundle.content().source().output_charge())
                            .max(bundle.content().output_charge());
                        assert_eq!(actual_peak, cap);
                    }
                    inspect_marked(bundle.content(), bundle.structure());
                    marked_probes::emit(
                        &format!("marked-driver-exact-{axis}"),
                        bundle.content(),
                        bundle.structure(),
                        &limits,
                        Some(bundle.encoded_prefix()),
                        Some(observed),
                        "marked_driver",
                        maximum,
                        bundle.constructor_charges(),
                        bundle.paints().reservation_sizes(),
                        Some(bundle.caller_before()),
                    );
                },
            );
            if short {
                assert!(result.is_err(), "axis{axis}: {result:?}");
                assert!(!delivered);
                let prefix = paint_observation(budget.observation());
                assert!(
                    marked_drive(&input, &limits, MODE, &mut budget, 4, |_, _, _| delivered =
                        true)
                    .is_err()
                );
                assert!(!delivered);
                let after = paint_observation(budget.observation());
                for i in [0, 1, 3] {
                    assert!(after[i] >= prefix[i]);
                }
            } else {
                assert!(result.is_ok(), "axis{axis}: {result:?}");
                assert!(delivered);
            }
            let actual = paint_observation(budget.observation())[axis];
            assert!(actual <= cap);
            if !short && axis != 2 {
                assert_eq!(actual, cap);
            }
        }
    }
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut before = [0; 4];
    let mut after = [0; 4];
    let result = marked_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, builder, observed| {
            before = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            let repeat = builder.build().unwrap();
            assert_eq!(repeat.fingerprint(), bundle.structure().fingerprint());
            after = [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps(),
            ];
            marked_probes::emit(
                "marked-driver-callback-before-repeat",
                bundle.content(),
                bundle.structure(),
                &caps,
                Some(bundle.encoded_prefix()),
                Some(observed),
                "marked_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.paints().reservation_sizes(),
                Some(bundle.caller_before()),
            );
            Err::<(), _>("marked callback sentinel")
        },
    );
    assert_eq!(result.unwrap(), Err("marked callback sentinel"));
    assert_eq!(paint_observation(budget.observation()), after);
    for i in [0, 1, 3] {
        assert!(after[i] > before[i]);
    }
    assert_eq!(after[2], before[2]);
    marked_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, _, observed| {
            assert_eq!(bundle.caller_before(), after);
            inspect_marked(bundle.content(), bundle.structure());
            marked_probes::emit(
                "marked-driver-cumulative-retry",
                bundle.content(),
                bundle.structure(),
                &caps,
                Some(bundle.encoded_prefix()),
                Some(observed),
                "marked_driver",
                MAXIMUM,
                bundle.constructor_charges(),
                bundle.paints().reservation_sizes(),
                Some(bundle.caller_before()),
            );
        },
    )
    .unwrap();
    assert!(
        budget.observation().record_charge() > after[0]
            && budget.observation().work_steps() > after[3]
    );
    let mut budget = BookV2ColumnPageBudget::new(&caps, after[3] - 1);
    let mut delivered = false;
    marked_drive(&input, &caps, MODE, &mut budget, 4, |_, builder, _| {
        delivered = true;
        assert!(matches!(
            builder.build(),
            Err(typaxis_pdf::book_v2::BookV2FontStreamError::Work)
        ));
        let prefix = [
            builder.record_charge(),
            builder.spool_charge(),
            builder.output_charge(),
            builder.work_steps(),
        ];
        assert!(builder.build().is_err());
        assert_eq!(
            [
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps()
            ],
            prefix
        );
    })
    .unwrap();
    assert!(delivered);
    assert_eq!(budget.observation().output_charge(), before[2]);
    assert!(budget.observation().work_steps() <= after[3] - 1);
}

#[test]
fn book_v2_column_marked_rejects_foreign_text_overlapping_objects_and_each_missing_text_prefix() {
    let caps = display_caps();
    let root = Root::new();
    let input=vector_input(&root,super::super::super::super::super::super::super::super::super::block_widths::column_block_data("vector",false,false),&caps);
    let mut outer = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    marked_drive(&input, &caps, MODE, &mut outer, 4, |bundle, _, _| {
        let content = bundle.content();
        let scopes = content.source();
        let text = content.text();
        let base = [
            scopes.record_charge(),
            scopes.spool_charge(),
            scopes.output_charge(),
            scopes.work_steps(),
        ];
        let changed = changed_caps(&caps, |c| c.max_fragments += 1);
        assert!(matches!(
            BookV2ColumnMarkedContentBuilder::new(scopes, text, &changed, MAXIMUM, 0, 0, 0, 0),
            Err(typaxis_pdf::book_v2::BookV2FontStreamError::Identity)
        ));
        let mut inner = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        marked_drive(&input, &caps, MODE, &mut inner, 4, |other, _, _| {
            assert!(!std::ptr::eq(bundle.display(), other.display()));
            let (mut r, mut s, mut o, mut w) = (0, 0, 0, 0);
            let result = BookV2ColumnMarkedContentBuilder::new_counted(
                scopes,
                other.content().text(),
                &caps,
                MAXIMUM,
                0,
                0,
                0,
                0,
                &mut r,
                &mut s,
                &mut o,
                &mut w,
            );
            assert!(matches!(
                result,
                Err(typaxis_pdf::book_v2::BookV2FontStreamError::Identity)
            ));
            assert_eq!([r, s, o, w], base);
        })
        .unwrap();
        let vectors = scopes.source().source().source();
        let mut ob = BookV2ColumnImageObjectBuilder::new(
            vectors,
            text.source().first_object(),
            &caps,
            MAXIMUM,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        let objects = ob.build().unwrap();
        let mut ib =
            BookV2ColumnImageCommandBuilder::new(&objects, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
        let commands = ib.build().unwrap();
        let mut sb =
            BookV2ColumnMarkedScopeBuilder::new(&commands, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
        let overlap = sb.build().unwrap();
        let (mut r, mut s, mut o, mut w) = (0, 0, 0, 0);
        assert!(matches!(
            BookV2ColumnMarkedContentBuilder::new_counted(
                &overlap, text, &caps, MAXIMUM, 0, 0, 0, 0, &mut r, &mut s, &mut o, &mut w
            ),
            Err(typaxis_pdf::book_v2::BookV2FontStreamError::Objects)
        ));
        assert_eq!(
            [r, s, o, w],
            [
                overlap.record_charge(),
                overlap.spool_charge(),
                overlap.output_charge(),
                overlap.work_steps()
            ]
        );
        let prefix = [
            text.record_charge(),
            text.spool_charge(),
            text.output_charge(),
            text.work_steps(),
        ];
        for axis in 0..4 {
            let mut p = prefix;
            p[axis] -= 1;
            let mut b = BookV2ColumnFontSelectionBuilder::new(
                bundle.display(),
                &caps,
                MAXIMUM,
                p[0],
                p[1],
                p[3],
            )
            .unwrap();
            let selected = b.select_images().unwrap();
            let raster = b.write_raster_programs(&selected).unwrap();
            let mut vb =
                BookV2ColumnVectorProgramBuilder::new(&raster, &caps, MAXIMUM, 0, 0, p[2], 0)
                    .unwrap();
            let vectors = vb.build().unwrap();
            if axis == 2 {
                assert_eq!(vectors.prior_output_charge(), prefix[2] - 1);
            } else {
                let value = selected.prior_charges()[if axis == 3 { 2 } else { axis }];
                assert_eq!(value, prefix[axis] - 1);
            }
            let mut ob = BookV2ColumnImageObjectBuilder::new(
                &vectors,
                text.source().next_object(),
                &caps,
                MAXIMUM,
                0,
                0,
                0,
                0,
            )
            .unwrap();
            let objects = ob.build().unwrap();
            let mut ib =
                BookV2ColumnImageCommandBuilder::new(&objects, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
            let commands = ib.build().unwrap();
            let mut sb =
                BookV2ColumnMarkedScopeBuilder::new(&commands, &caps, MAXIMUM, 0, 0, 0, 0).unwrap();
            let missing = sb.build().unwrap();
            let (mut r, mut s, mut o, mut w) = (0, 0, 0, 0);
            let result = BookV2ColumnMarkedContentBuilder::new_counted(
                &missing, text, &caps, MAXIMUM, 0, 0, 0, 0, &mut r, &mut s, &mut o, &mut w,
            );
            assert!(
                matches!(
                    result,
                    Err(typaxis_pdf::book_v2::BookV2FontStreamError::Identity)
                ),
                "axis{axis}"
            );
            assert_eq!(
                [r, s, o, w],
                [
                    missing.record_charge(),
                    missing.spool_charge(),
                    missing.output_charge(),
                    missing.work_steps()
                ]
            );
        }
    })
    .unwrap();
}

fn manual_marked(
    case: &str,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
) -> MarkedFacts {
    let mut constructors = [[0; 4]; 11];
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
    let mut b = BookV2ColumnFontStreamBuilder::new(&cids, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[1] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let streams = b.build().unwrap();
    let mut b = BookV2ColumnFontObjectBuilder::new(&streams, 4, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[2] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let objects = b.build().unwrap();
    let mut b = BookV2ColumnTextCommandBuilder::new(&objects, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[3] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let text = b.build().unwrap();
    let mut image_builder = fonts
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
        image_builder.record_charge(),
        image_builder.spool_charge(),
        text.output_charge(),
        image_builder.work_steps(),
    ];
    let images = image_builder.select_images().unwrap();
    let rasters = image_builder.write_raster_programs(&images).unwrap();
    let mut b = BookV2ColumnVectorProgramBuilder::new(
        &rasters,
        caps,
        MAXIMUM,
        0,
        0,
        text.output_charge(),
        0,
    )
    .unwrap();
    constructors[5] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let vectors = b.build().unwrap();
    let mut b = BookV2ColumnImageObjectBuilder::new(
        &vectors,
        objects.next_object(),
        caps,
        MAXIMUM,
        0,
        0,
        0,
        0,
    )
    .unwrap();
    constructors[6] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let objects = b.build().unwrap();
    let mut b = BookV2ColumnImageCommandBuilder::new(&objects, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[7] = [
        b.record_charge(),
        b.spool_charge(),
        b.output_charge(),
        b.work_steps(),
    ];
    let commands = b.build().unwrap();
    let mut sb = BookV2ColumnMarkedScopeBuilder::new(&commands, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[8] = [
        sb.record_charge(),
        sb.spool_charge(),
        sb.output_charge(),
        sb.work_steps(),
    ];
    let scopes = sb.build().unwrap();
    let mut cb =
        BookV2ColumnMarkedContentBuilder::new(&scopes, &text, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[9] = [
        cb.record_charge(),
        cb.spool_charge(),
        cb.output_charge(),
        cb.work_steps(),
    ];
    let content = cb.build().unwrap();
    let mut st =
        BookV2ColumnSourceStructureBuilder::new(&content, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[10] = [
        st.record_charge(),
        st.spool_charge(),
        st.output_charge(),
        st.work_steps(),
    ];
    let structure = st.build().unwrap();
    let facts = inspect_marked(&content, &structure);
    marked_probes::emit(
        case,
        &content,
        &structure,
        caps,
        observed,
        observed,
        scope,
        MAXIMUM,
        constructors,
        units,
        None,
    );
    facts
}

#[test]
fn book_v2_column_marked_retained_cut_gid71_is_artifact_only_with_original_source_nodes_once() {
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
        let facts = manual_marked(
            "marked-contextual-retained-cut",
            None,
            "retained_source_cut",
            display,
            caps,
        );
        assert!(facts.artifacts > 0 && facts.groups > facts.bindings && facts.anchors > 0);
    });
}

#[test]
fn book_v2_column_marked_manual_source_callback_keeps_owned_native_and_vector_hierarchy() {
    let caps = display_caps();
    for kind in ["native", "vector"] {
        let data=super::super::super::super::super::super::super::super::super::block_widths::column_block_data(kind,true,true);
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
            let mut db =
                BookV2ColumnDisplayBuilder::new(source, input.resources(), &caps, MAXIMUM, 0, 0)
                    .unwrap();
            let display = db.build().unwrap();
            let facts = manual_marked(
                &format!("marked-manual-source-{kind}"),
                Some(observed),
                "source_callback",
                &display,
                &caps,
            );
            assert!(facts.formulas > 0 && facts.artifacts > 0);
            assert_eq!(facts.anchors > 0, kind == "vector");
        })
        .unwrap();
    }
}

#[test]
#[ignore = "requires TYPAXIS_HARANO_FONT pointing to original HaranoAjiMincho-Regular.otf SHA256 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"]
fn book_v2_column_marked_original_harano_cid_cff_source_hierarchy() {
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
    let facts = input_marked("marked-original-harano-cid-cff", &input, &caps);
    assert!(facts.semantic > 0 && facts.artifacts > 0);
}
