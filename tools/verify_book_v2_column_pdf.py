#!/usr/bin/env python3
"""Reconstruct inspectable Column PDF assembly from authentic source roots.

The old parser cores remain unchanged. Their command-line Body guard is never
used here; actual Column frames, source metadata and running regions are checked
separately. This is not PDF/UA or whole-book acceptance.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import importlib
import io
import json
import re
import sys
from collections import Counter
from pathlib import Path

import verify_book_v2_column_paints as paints
import verify_book_v2_column_navigation as navigation

check = paints.check
fields = paints.fields


def strict_equal(actual, expected, label):
    if actual is expected:
        return
    paints.strict_equal(actual, expected, label)


integer = paints.integer
artifact = paints.artifact
PDFBudget = paints.PDFBudget
SCHEMA = "typaxis.book-2-column-pdf-probe/1"
ALGORITHM = b"typaxis.book-2-pdf-assembly/1"


def sha(value):
    return hashlib.sha256(value).hexdigest()


def configure_pdf_site(site=None):
    """Load the existing pure-Python pypdf package without installing anything."""
    if site is not None:
        site = Path(site).resolve()
        check((site / "pypdf/__init__.py").is_file(), "existing explicit pypdf package")
        if str(site) not in sys.path:
            sys.path.insert(1, str(site))
    package = importlib.import_module("pypdf")
    check(package.__version__ == "6.10.0", "unchanged previously pinned pypdf 6.10.0")
    base = Path(package.__file__).resolve().parent
    if site is not None:
        check(
            base == site / "pypdf", "pypdf loaded from declared unchanged environment"
        )
    files = {
        str(path.relative_to(base)): sha(path.read_bytes())
        for path in sorted(base.rglob("*"))
        if path.is_file() and "__pycache__" not in path.parts
    }
    return {"version": package.__version__, "base": str(base), "files": files}


class Encoder(paints.Sink):
    def __init__(self, budget, retained=True):
        self.budget = budget
        self.bytes = bytearray() if retained else None
        self.length = 0

    def extend(self, value):
        check(isinstance(value, bytes), "closed PDF byte contribution")
        self.budget.step((len(value) + 63) // 64 + 1)
        self.length += len(value)
        if self.bytes is not None:
            self.bytes.extend(value)

    def reference(self, value):
        self.unsigned(value)
        self.extend(b" 0 R ")

    def text(self, value):
        self.utf16(value, True)

    def coordinate(self, value):
        self.number(value)
        self.push(32)

    def structure_id(self, value):
        self.hex(integer(value, 32, False).to_bytes(4, "big"))

    def fixed(self, value, width):
        check(0 <= value < 10**width, "bounded PDF fixed decimal")
        self.extend(f"{value:0{width}d}".encode())

    def stream(self, attributes, body):
        counter = Encoder(self.budget, False)
        body(counter)
        self.extend(b"<< /Length ")
        self.unsigned(counter.length)
        self.extend(attributes)
        self.extend(b" >>\nstream\n")
        start = self.length
        body(self)
        check(
            self.length - start == counter.length,
            "exact nested stream measure/write length",
        )
        self.extend(b"\nendstream")


def xml(out, value, attribute=False):
    for character in value:
        escaped = {"&": "&amp;", "<": "&lt;", ">": "&gt;"}.get(character)
        if character == '"' and attribute:
            escaped = "&quot;"
        out.extend((escaped if escaped is not None else character).encode())


def xmp(out, metadata, language, engine):
    out.extend(
        b'<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/">'
    )

    def alt(property, value):
        # Rust fmt::Write emits the static and dynamic pieces separately.
        out.extend(b"<")
        out.extend(property.encode())
        out.extend(b'><rdf:Alt><rdf:li xml:lang="x-default">')
        xml(out, value)
        out.extend(b"</rdf:li>")
        if language != "x-default":
            out.extend(b'<rdf:li xml:lang="')
            xml(out, language, True)
            out.extend(b'">')
            xml(out, value)
            out.extend(b"</rdf:li>")
        out.extend(b"</rdf:Alt></")
        out.extend(property.encode())
        out.extend(b">")

    if metadata["title"] is not None:
        alt("dc:title", metadata["title"])
    if metadata["author"] is not None:
        out.extend(b"<dc:creator><rdf:Seq><rdf:li>")
        xml(out, metadata["author"])
        out.extend(b"</rdf:li></rdf:Seq></dc:creator>")
    if metadata["subject"] is not None:
        alt("dc:description", metadata["subject"])
    if metadata["keywords"]:
        out.extend(b"<dc:subject><rdf:Bag>")
        for value in metadata["keywords"]:
            out.extend(b"<rdf:li>")
            xml(out, value)
            out.extend(b"</rdf:li>")
        out.extend(b"</rdf:Bag></dc:subject><pdf:Keywords>")
        for index, value in enumerate(metadata["keywords"]):
            if index:
                out.extend(b"; ")
            xml(out, value)
        out.extend(b"</pdf:Keywords>")
    for field, property in (
        ("identifier", "dc:identifier"),
        ("created", "xmp:CreateDate"),
        ("modified", "xmp:ModifyDate"),
    ):
        value = metadata[field]
        if value is not None:
            out.extend(("<" + property + ">").encode())
            if field == "identifier":
                xml(out, value)
            else:
                out.extend(value.encode())
            out.extend(("</" + property + ">").encode())
    out.extend(b"<dc:language><rdf:Bag><rdf:li>")
    xml(out, language)
    out.extend(b"</rdf:li></rdf:Bag></dc:language><pdf:Producer>")
    xml(out, engine["name"])
    out.extend(b" ")
    xml(out, engine["version"])
    out.extend(b"</pdf:Producer>")
    out.extend(b"</rdf:Description></rdf:RDF></x:xmpmeta>")


def info(out, metadata, engine):
    out.extend(b"<<")
    for key, field in (
        (b" /Title ", "title"),
        (b" /Author ", "author"),
        (b" /Subject ", "subject"),
    ):
        if metadata[field] is not None:
            out.extend(key)
            out.text(metadata[field])
    if metadata["keywords"]:
        out.extend(b" /Keywords ")
        out.text("; ".join(metadata["keywords"]))
    for key, field in ((b" /CreationDate ", "created"), (b" /ModDate ", "modified")):
        if metadata[field] is not None:
            value = metadata[field].encode()
            check(len(value) == 20 and value.isascii(), "original admitted date")
            out.extend(key)
            out.extend(b"(D:")
            for start, end in ((0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)):
                out.extend(value[start:end])
            out.extend(b"Z)")
    out.extend(b" /Producer ")
    out.text(engine["name"] + " " + engine["version"])
    out.extend(b" >>")


def object_plan(first, pages, nodes, annotations, outlines, names, maximum):
    cursor = integer(first, 32, False)

    def allocate(count):
        nonlocal cursor
        start = cursor
        cursor += count
        check(
            cursor <= 0xFFFFFFFF and cursor - 1 <= maximum,
            "actual bounded PDF object cursor",
        )
        return start

    allocate(8)
    result = dict(
        first=first,
        page=allocate(pages * 2),
        node=allocate(nodes),
        annotation=allocate(annotations),
        outline=allocate(outlines),
    )
    result["dests"] = allocate(1) if names else None
    result["outlines"] = allocate(1) if outlines else None
    result["next"] = cursor
    return result


def encode(probe, plan, masters, names, node_links, budget, retained=True):
    source = probe["structural_probe"]
    wire, nodes, bindings, groups = (
        source[key] for key in ("wire", "nodes", "bindings", "groups")
    )
    relations, nav = source["relations"], source["relations"]["navigation"]
    fonts, images, marked = (
        probe[key] for key in ("font_objects", "image_objects", "marked")
    )
    payload = probe["_payload"]
    metadata, engine = wire["metadata"], probe["engine"]
    out = Encoder(budget, retained)
    objects = [None] * plan["next"]
    page_id = lambda i: plan["page"] + 2 * i
    node_id = lambda i: plan["node"] + i
    annotation_id = lambda i: plan["annotation"] + i
    outline_id = lambda i: plan["outline"] + i
    fixed_id = lambda i: plan["first"] + i

    def raw(id, value):
        check(
            0 < id < len(objects) and objects[id] is None,
            "unique occupied raw PDF object",
        )
        start = out.length
        out.extend(value)
        objects[id] = [start, out.length]

    def obj(id, body):
        check(
            0 < id < len(objects) and objects[id] is None, "unique bounded PDF object"
        )
        start = out.length
        out.unsigned(id)
        out.extend(b" 0 obj\n")
        body(out)
        out.extend(b"\nendobj\n")
        objects[id] = [start, out.length]

    def destination(sink, index):
        page, fragment, x, y = nav["destinations"][index]["position"]
        sink.push(91)
        sink.reference(page_id(page))
        sink.extend(b" /XYZ ")
        sink.coordinate(x)
        sink.coordinate(masters[page]["height"] - y)
        sink.extend(b"null]")

    out.extend(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    for kind in ("font_objects", "image_objects"):
        for record in probe[kind]["objects"]:
            start, end = record["range"]
            raw(record["id"], payload[kind][start:end])
    for record in marked["anchor_objects"]:
        start, end = record["range"]
        raw(record["id"], payload["anchor_bytes"][start:end])

    def catalog(s):
        s.extend(b"<< /Type /Catalog /Pages ")
        s.reference(fixed_id(1))
        s.extend(b" /Lang ")
        s.text(nodes[0]["language"])
        s.extend(
            b" /MarkInfo << /Marked true >> /ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot "
        )
        s.reference(fixed_id(5))
        s.extend(b" /Metadata ")
        s.reference(fixed_id(3))
        if plan["dests"] is not None:
            s.extend(b" /Names << /Dests ")
            s.reference(plan["dests"])
            s.extend(b" >>")
        if plan["outlines"] is not None:
            s.extend(b" /Outlines ")
            s.reference(plan["outlines"])
        s.extend(b" >>")

    obj(fixed_id(0), catalog)

    def pages_body(s):
        s.extend(b"<< /Type /Pages /Count ")
        s.unsigned(len(masters))
        s.extend(b" /Kids [")
        for page in range(len(masters)):
            s.reference(page_id(page))
        s.extend(b"] >>")

    obj(fixed_id(1), pages_body)
    obj(fixed_id(2), lambda s: info(s, metadata, engine))
    obj(
        fixed_id(3),
        lambda s: s.stream(
            b" /Type /Metadata /Subtype /XML",
            lambda nested: xmp(nested, metadata, nodes[0]["language"], engine),
        ),
    )

    def resources(s):
        s.extend(b"<< /Font <<")
        for record in fonts["resources"]:
            s.extend(b" /PB")
            s.unsigned(record["instance"])
            s.push(32)
            s.reference(record["object"])
        if marked["anchor_font"] is not None:
            s.extend(b" /BMA ")
            s.reference(marked["anchor_font"])
        s.extend(b" >> /XObject <<")
        for index, record in enumerate(images["resources"]):
            s.extend(b" /BI")
            s.unsigned(index)
            s.push(32)
            s.reference(record["object"])
        s.extend(b" >> >>")

    obj(fixed_id(4), resources)
    for index, master in enumerate(masters):

        def page_body(s):
            s.extend(b"<< /Type /Page /Parent ")
            s.reference(fixed_id(1))
            s.extend(b" /MediaBox [0 0 ")
            s.coordinate(master["width"])
            s.coordinate(master["height"])
            s.extend(b"] /TrimBox [")
            trim = master["trim"]
            for value in (
                trim["x"],
                master["height"] - trim["y"] - trim["height"],
                trim["x"] + trim["width"],
                master["height"] - trim["y"],
            ):
                s.coordinate(value)
            s.extend(b"] /Resources ")
            s.reference(fixed_id(4))
            s.extend(b" /Contents ")
            s.reference(page_id(index) + 1)
            s.extend(b" /StructParents ")
            s.unsigned(index)
            s.extend(b" /Tabs /S")
            rectangles = [
                i for i, r in enumerate(nav["rectangles"]) if r["page"] == index
            ]
            if rectangles:
                s.extend(b" /Annots [")
                for i in rectangles:
                    s.reference(annotation_id(i))
                s.push(93)
            s.extend(b" >>")

        obj(page_id(index), page_body)

        def content(s):
            s.extend(b"q\n1 0 0 -1 0 ")
            s.coordinate(master["height"])
            s.extend(b"cm\n")
            s.extend(payload["marked_pages"][index])
            s.extend(b"Q\n")

        obj(page_id(index) + 1, lambda s: s.stream(b"", content))

    def structure(s):
        s.extend(b"<< /Type /StructTreeRoot /K [")
        s.reference(node_id(0))
        s.extend(b"] /ParentTree ")
        s.reference(fixed_id(6))
        s.extend(b" /ParentTreeNextKey ")
        s.unsigned(len(masters) + len(nav["rectangles"]))
        s.extend(b" /IDTree ")
        s.reference(fixed_id(7))
        s.extend(b" >>")

    obj(fixed_id(5), structure)

    def parents(s):
        s.extend(b"<< /Nums [")
        bound = {row["group"]: row for row in bindings}
        for page in range(len(masters)):
            s.unsigned(page)
            s.extend(b" [")
            for index in range(*marked["pages"][page]["groups"]):
                s.budget.step(1)
                if index in bound:
                    s.reference(node_id(bound[index]["node"]))
            s.extend(b"] ")
        for index, rect in enumerate(nav["rectangles"]):
            s.unsigned(len(masters) + index)
            s.push(32)
            s.reference(node_id(nav["links"][rect["link"]]["node"]))
        s.extend(b"] >>")

    obj(fixed_id(6), parents)

    def ids(s):
        s.extend(b"<< /Names [")
        for index in range(len(nodes)):
            s.structure_id(index)
            s.push(32)
            s.reference(node_id(index))
        s.extend(b"] >>")

    obj(fixed_id(7), ids)
    for index, node in enumerate(nodes):
        rel = relations["nodes"][index]

        def node_body(s):
            s.extend(b"<< /Type /StructElem /S /")
            s.extend(node["role"].encode())
            s.extend(b" /P ")
            s.reference(
                fixed_id(5) if rel["parent"] is None else node_id(rel["parent"])
            )
            s.extend(b" /Lang ")
            s.text(node["language"])
            s.extend(b" /ID ")
            s.structure_id(index)
            if node["alt"] is not None:
                s.extend(b" /Alt ")
                s.text(node["alt"])
            if node["semantic_kind"] is not None:
                s.extend(b" /TypaxisSourceKind ")
                s.text(node["semantic_kind"])
            if rel["list_numbering"] is not None:
                s.extend(b" /A << /O /List /ListNumbering /")
                s.extend(rel["list_numbering"].encode())
                s.extend(b" >>")
            if node["cell"] is not None:
                cell = node["cell"]
                s.extend(b" /A << /O /Table")
                if cell["header"]:
                    s.extend(b" /Scope /Column")
                if cell["rowspan"] > 1:
                    s.extend(b" /RowSpan ")
                    s.unsigned(cell["rowspan"])
                if cell["colspan"] > 1:
                    s.extend(b" /ColSpan ")
                    s.unsigned(cell["colspan"])
                if rel["headers"]:
                    s.extend(b" /Headers [")
                    for header in rel["headers"]:
                        s.structure_id(header)
                        s.push(32)
                    s.push(93)
                s.extend(b" >>")
            if rel["related"]:
                s.extend(b" /Ref [")
                for related in rel["related"]:
                    s.reference(node_id(related))
                s.push(93)
            s.extend(b" /K [")
            cursor = node["first_binding"]
            while cursor is not None:
                s.budget.step(1)
                binding = bindings[cursor]
                s.extend(b"<< /Type /MCR /Pg ")
                s.reference(page_id(binding["page"]))
                s.extend(b" /MCID ")
                s.unsigned(binding["mcid"])
                s.extend(b" >> ")
                cursor = binding["next"]
            cursor = rel["first_child"]
            while cursor is not None:
                s.budget.step(1)
                s.reference(node_id(cursor))
                cursor = relations["nodes"][cursor]["next"]
            link = node_links[index]
            if link is not None:
                cursor = nav["links"][link]["first"]
                while cursor is not None:
                    s.budget.step(1)
                    rect = nav["rectangles"][cursor]
                    s.extend(b"<< /Type /OBJR /Pg ")
                    s.reference(page_id(rect["page"]))
                    s.extend(b" /Obj ")
                    s.reference(annotation_id(cursor))
                    s.extend(b" >> ")
                    cursor = rect["next"]
            s.extend(b"] >>")

        obj(node_id(index), node_body)
    for index, rect in enumerate(nav["rectangles"]):

        def annotation(s):
            x, y, width, height = rect["bounds"]
            page = rect["page"]
            master = masters[page]
            s.extend(b"<< /Type /Annot /Subtype /Link /F 4 /Border [0 0 0] /Rect [")
            for value in (
                x,
                master["height"] - y - height,
                x + width,
                master["height"] - y,
            ):
                s.coordinate(value)
            s.extend(b"] /P ")
            s.reference(page_id(page))
            s.extend(b" /StructParent ")
            s.unsigned(len(masters) + index)
            target = nav["links"][rect["link"]]["target"]
            if target[0] == "destination":
                s.extend(b" /Dest ")
                destination(s, target[1])
            else:
                check(target[0] == "uri", "closed original link target")
                s.extend(b" /A << /S /URI /URI ")
                s.hex(target[1].encode())
                s.extend(b" >>")
            s.extend(b" >>")

        obj(annotation_id(index), annotation)
    if plan["dests"] is not None:

        def destinations(s):
            s.extend(b"<< /Names [")
            for index in names:
                s.budget.step(1)
                s.hex(nav["destinations"][index]["kind"][1].encode())
                s.push(32)
                destination(s, index)
                s.push(32)
            s.extend(b"] >>")

        obj(plan["dests"], destinations)
    if plan["outlines"] is not None:

        def outline_root(s):
            s.extend(b"<< /Type /Outlines /Count ")
            s.unsigned(len(nav["outline"]))
            s.extend(b" /First ")
            s.reference(outline_id(0))
            last = 0
            for index, item in enumerate(nav["outline"]):
                s.budget.step(1)
                if item["parent"] is None:
                    last = index
            s.extend(b" /Last ")
            s.reference(outline_id(last))
            s.extend(b" >>")

        obj(plan["outlines"], outline_root)
    for index, item in enumerate(nav["outline"]):

        def outline_body(s):
            s.extend(b"<< /Title ")
            s.text(nav["source_outline"][index]["label"])
            s.extend(b" /Parent ")
            s.reference(
                plan["outlines"]
                if item["parent"] is None
                else outline_id(item["parent"])
            )
            for key, field in (
                (b" /Prev", "previous"),
                (b" /Next", "next"),
                (b" /First", "first"),
                (b" /Last", "last"),
            ):
                if item[field] is not None:
                    s.extend(key)
                    s.push(32)
                    s.reference(outline_id(item[field]))
            if item["first"] is not None:
                s.extend(b" /Count ")
                s.unsigned(item["descendants"])
            s.extend(b" /Dest ")
            destination(s, item["destination"])
            s.extend(b" >>")

        obj(outline_id(index), outline_body)
    xref = out.length
    out.extend(b"xref\n0 ")
    out.unsigned(len(objects))
    out.push(10)
    out.budget.step(len(objects))
    free = next((i for i, value in enumerate(objects[1:], 1) if value is None), 0)
    out.fixed(free, 10)
    out.extend(b" 65535 f \n")
    free_cursor = 1
    for index, value in enumerate(objects[1:], 1):
        out.budget.step(1)
        if value is None:
            free_cursor = max(free_cursor, index + 1)
            while free_cursor < len(objects) and objects[free_cursor] is not None:
                out.budget.step(1)
                free_cursor += 1
            out.fixed(0 if free_cursor == len(objects) else free_cursor, 10)
            out.extend(b" 00000 f \n")
        else:
            out.fixed(value[0], 10)
            out.extend(b" 00000 n \n")
    out.extend(b"trailer\n<< /Size ")
    out.unsigned(len(objects))
    out.extend(b" /Root ")
    out.reference(fixed_id(0))
    out.extend(b" /Info ")
    out.reference(fixed_id(2))
    out.extend(b" >>\nstartxref\n")
    out.unsigned(xref)
    out.extend(b"\n%%EOF\n")
    return out, objects


def reserve(budget, count, unit):
    budget.reserve(count, count * unit, 0)
    budget.step(1)


def master_choice(core, wire, page, name, budget):
    masters = wire["page_masters"]
    chosen = core.selected_master(masters, page, name)
    budget.step(len(masters["selection_rules"]))
    ids = [master["master_id"] for master in masters["masters"]]
    check(ids == sorted(set(ids)), "original sorted unique master identities")
    for _ in range(2):
        low, high = 0, len(ids)
        while low < high:
            budget.step(1)
            middle = low + (high - low) // 2
            if ids[middle] == chosen["master_id"]:
                break
            if ids[middle] < chosen["master_id"]:
                low = middle + 1
            else:
                high = middle
        else:
            raise AssertionError("selected original base/advanced master exists")
    return chosen


def assembly_replay(core, root, probe):
    source = probe["structural_probe"]
    nav, nodes = source["relations"]["navigation"], source["nodes"]
    wire, stage, observed, sizes = (
        source["wire"],
        probe["assembly"],
        probe["observed"],
        probe["size_of"],
    )
    budget = PDFBudget(probe["navigation"]["charges"])
    budget.reserve(1, 0, 0)
    budget.matches4(observed["constructors"][13])
    budget.step(1)
    reserve(budget, len(probe["pages"]), sizes["selected_page_master"])
    masters = []
    for index, page in enumerate(probe["pages"]):
        master = master_choice(core, wire, index, page["named_page"], budget)
        budget.step(
            int(master["header_content"] is not None)
            + int(master["footer_content"] is not None)
        )
        # The Column frame view authenticates the original named master again.
        strict_equal(
            master_choice(core, wire, index, page["named_page"], budget),
            master,
            "same source master during frame validation",
        )
        budget.step(len(page["columns"]))
        masters.append(master)
    names = [
        i
        for i, value in enumerate(nav["destinations"])
        if value["kind"][0] == "anchor" and value["position"] is not None
    ]
    reserve(budget, len(nav["destinations"]), sizes["usize"])
    budget.step(len(nav["destinations"]))
    for left, right in zip(names, names[1:]):
        a, b = (
            nav["destinations"][left]["kind"][1].encode(),
            nav["destinations"][right]["kind"][1].encode(),
        )
        budget.step(1 + min(len(a), len(b)))
        check(a < b, "placed anchor PDF key byte ordering")
    reserve(budget, len(nodes), sizes["option_usize"])
    budget.step(len(nodes))
    node_links = [None] * len(nodes)
    for index, link in enumerate(nav["links"]):
        budget.step(1)
        check(
            node_links[link["node"]] is None,
            "one active navigation link per original structure node",
        )
        node_links[link["node"]] = index
    facts = navigation.original_facts(
        {
            "relation_probe": {
                "marked_probe": {
                    "source_package": {"wire": wire},
                    "source_structure": {"nodes": nodes},
                }
            }
        }
    )
    count = 0
    for paragraph in facts["paragraphs"]:
        budget.step(1)
        for site in paragraph["items"]:
            budget.step(1)
            ref = site["reference"]
            count += int(
                ref is not None and ref["kind"] == "anchor" and ref["format"] == "page"
            )
    check(
        count == len(stage["candidates"]),
        "original Page-reference paragraph/site census",
    )
    reserve(budget, count, sizes["page_reference"])
    lookup = sorted((tuple(node["key"]), index) for index, node in enumerate(nodes))
    for owner, candidate in stage["candidates"]:
        budget.step(1)
        # Navigation.find uses source-slot ordinal ordering, not role spelling.
        key = (owner, navigation.marked.SLOTS.index("ReferenceLink"))
        ordered = sorted(
            ((item[0][0], navigation.marked.SLOTS.index(item[0][1])), item[1])
            for item in lookup
        )
        low, high = 0, len(ordered)
        while low < high:
            budget.step(1)
            middle = low + (high - low) // 2
            if ordered[middle][0] == key:
                break
            if ordered[middle][0] < key:
                low = middle + 1
            else:
                high = middle
        else:
            raise AssertionError(
                "authored Page reference has original ReferenceLink slot"
            )
    caps = json.loads(probe["limits_canonical_jcs"])["base"]
    plan = object_plan(
        probe["marked"]["next_object"],
        len(masters),
        len(nodes),
        len(nav["rectangles"]),
        len(nav["outline"]),
        names,
        caps["max_pdf_objects"],
    )
    reserve(budget, plan["next"], sizes["object_range"])
    budget.step(plan["next"])
    credit = (
        len(probe["_payload"]["font_objects"])
        + len(probe["_payload"]["image_objects"])
        + probe["marked"]["byte_length"]
    )
    check(
        budget.output >= credit,
        "authentic already-paid source contributions cover assembly credit",
    )
    measured, measured_objects = encode(
        probe, plan, masters, names, node_links, budget, False
    )
    budget.reserve(0, measured.length, max(0, measured.length - credit))
    if measured.length < credit:
        budget.output -= credit - measured.length
    budget.step(plan["next"])
    encoded, objects = encode(probe, plan, masters, names, node_links, budget)
    check(
        encoded.length == measured.length and objects == measured_objects,
        "independent complete PDF two-pass encoding",
    )
    fp = budget.fold(bytes.fromhex(probe["navigation"]["fingerprint"]), ALGORITHM)
    data = bytes(encoded.bytes)
    for first in range(0, len(data), 96):
        fp = budget.fold(fp, data[first : first + 96])
    for ref in stage["references"]:
        value = b"".join(
            integer(ref[key], 32, False).to_bytes(4, "big")
            for key in ("owner", "candidate")
        )
        value += integer(ref["target"] or 0, 32, False).to_bytes(4, "big") + bytes(
            [ref["placed"]]
        )
        fp = budget.fold(fp, value)
    return dict(
        data=data,
        objects=objects,
        plan=plan,
        budget=budget,
        fingerprint=fp.hex(),
        replacement_credit=credit,
    )


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def bounds(value):
    fields(value, ("x", "y", "width", "height"), "original authored rectangle")
    return [value[key] for key in ("x", "y", "width", "height")]


def verify_frames(core, probe):
    parent, source = probe["parent"], probe["structural_probe"]
    wire, pages = source["wire"], probe["pages"]
    check(len(pages) == len(parent["pages"]), "complete parent physical page census")
    actual = source["relations"]["navigation"]["assembly"]
    strict_equal(
        actual["page_names"],
        [p["named_page"] for p in pages],
        "same physical source page requests",
    )
    strict_equal(
        actual["page_frames"],
        [p["full_body"] for p in pages],
        "same full physical source bodies",
    )
    strict_equal(
        actual["footnote_regions"],
        [p["declared_note"] for p in pages],
        "same declared note region occurrences",
    )
    source_pages = parent["base_display"]["pages"]
    for index, (page, earlier) in enumerate(zip(pages, parent["pages"])):
        fields(
            page,
            (
                "index",
                "named_page",
                "named_page_index",
                "master_id",
                "full_body",
                "declared_note",
                "columns",
                "balance",
            ),
            "complete Column physical frame",
        )
        check(
            page["index"] == index == earlier["index"],
            "dense physical page order including blanks",
        )
        for a, b in (
            ("named_page", "named_page"),
            ("master_id", "selected_master_id"),
            ("full_body", "full_body"),
            ("declared_note", "declared_note"),
            ("columns", "columns"),
        ):
            strict_equal(page[a], earlier[b], "unchanged parent physical " + a)
        if source_pages:
            strict_equal(
                page["named_page_index"],
                source_pages[index]["named_index"],
                "original selected source-page identity",
            )
        master = core.selected_master(wire["page_masters"], index, page["named_page"])
        strict_equal(
            page["master_id"],
            master["master_id"],
            "ranked actual first/parity/named master",
        )
        strict_equal(
            page["full_body"], bounds(master["body"]), "original full-width master body"
        )
        strict_equal(
            page["declared_note"],
            None if master["footnote"] is None else bounds(master["footnote"]),
            "original declared note rectangle",
        )
        layout = master["column_layout"]
        check(
            layout is not None and layout["fill"] == "sequential",
            "genuine declared sequential Column master",
        )
        count, gap = layout["count"], layout["gap"]
        x, y, width, height = page["full_body"]
        available = width - gap * (count - 1)
        check(available > 0 and count >= 2, "positive original multi-column widths")
        unit, remainder = divmod(available, count)
        balance = page["balance"]
        if balance is not None:
            fields(
                balance,
                ("fingerprint", "full_height", "selected_height"),
                "actual last-page balance receipt link",
            )
            check(
                layout["balance"] == "last_page"
                and balance["full_height"] == height
                and 0 <= balance["selected_height"] <= height,
                "actual balanced capacity inside original full frame",
            )
            strict_equal(
                balance["fingerprint"],
                earlier["balance_fingerprint"],
                "same sealed balance receipt",
            )
            height = balance["selected_height"]
        else:
            check(
                earlier["balance_fingerprint"] is None, "no fabricated balance receipt"
            )
        expected = [
            [
                x + i * (unit + gap),
                y,
                unit + (remainder if i + 1 == count else 0),
                height,
            ]
            for i in range(count)
        ]
        strict_equal(
            page["columns"],
            expected,
            "exact original Column partition with final-column raw-unit residual",
        )
    core.source_page_names(source)


def verify_marked_bytes(root, probe, regions):
    """Reconstruct every local marked group from verified joined paint commands.

    This reuses the original scope projection rules over the real seven-stream
    joined display. Running-region draws remain artifacts and retain their own
    original source graph. No resource or layout authority is synthesized.
    """
    parent, source, stage = probe["parent"], probe["structural_probe"], probe["marked"]
    m = navigation.marked
    scratch = PDFBudget([0, 0, 0, 0])
    _, locators = regions.verify_join(parent)
    base = parent["base_display"]
    ordinary = m.properties(dict(paint_probe=dict(display_probe=base)))
    properties = {tuple(p): v for p, v in zip(base["paints"], ordinary)}
    props = []
    for tag, index in parent["paints"]:
        if tag == "region":
            ri, di = locators[index]
            r, draw = parent["regions"][ri], parent["regions"][ri]["draws"][di]
            props.append(
                dict(
                    owner=draw["owner"],
                    page=r["page"],
                    fragment=ri,
                    role="Text",
                    artifact=(
                        "RunningHeader" if r["kind"] == "header" else "RunningFooter"
                    ),
                    alt=None,
                    actual=None,
                    stream="region",
                    draw=index,
                )
            )
        else:
            props.append(properties[(tag, index)])
    groups = m.visit_groups(props, scratch)
    check(
        len(groups) == len(source["groups"]) == len(stage["groups"]),
        "all source and running artifact marked groups",
    )
    text = parent["text_commands"]
    text_data = artifact(root, text["bytes"])
    text_by_paint = [[] for _ in parent["paints"]]
    for command in text["commands"]:
        text_by_paint[command["paint"]].append(command)
    image = parent["image_commands"]
    image_data = artifact(root, image["bytes"])
    image_by_paint = {c["paint"]: c for c in image["commands"]}
    check(
        len(image_by_paint) == len(image["commands"]),
        "one actual image command per distinct paint occurrence",
    )
    adapter = dict(paint_probe=dict(display_probe=dict(base, paints=parent["paints"])))
    sink = paints.Sink(scratch)
    pages, ranges, anchors = [], [], []
    cursor = text_cursor = paint_cursor = image_cursor = 0
    for page in range(len(probe["pages"])):
        start, group_start = len(sink.bytes), cursor
        sink.extend(b"q\n")
        while cursor < len(groups) and groups[cursor]["page"] == page:
            g = groups[cursor]
            start_group = len(sink.bytes)
            strict_equal(
                {k: source["groups"][cursor][k] for k in ("owner", "role", "page")},
                {k: g[k] for k in ("owner", "role", "page")},
                "actual group source identity and role",
            )
            sink.extend(b"q\n")
            m.begin_bytes(g, sink)
            inline = g["artifact"] is None and g["role"] in (
                "Text",
                "Label",
                "EquationNumber",
            )
            if inline:
                check(g["actual"] is None, "source replacement scopes do not nest")
                sink.extend(b"/Span << /ActualText ")
                sink.utf16(m.whole_text(adapter, g), True)
                sink.extend(b" >> BDC\n")
            anchor = m.anchor_for(adapter, cursor, g)
            if anchor is not None:
                m.encode_anchor_command(sink, anchor)
                anchors.append(anchor)
            check(
                g["paints"][0] == paint_cursor,
                "all original paints consume an ordered contiguous group",
            )
            for p in range(*g["paints"]):
                for command in text_by_paint[p]:
                    strict_equal(
                        command,
                        text["commands"][text_cursor],
                        "complete original joined text-command order",
                    )
                    check(
                        command["page"] == page
                        and (
                            command["usage"] is None
                            or inline
                            or g["actual"] is not None
                            or g["artifact"] is not None
                        ),
                        "original extraction owner and page",
                    )
                    sink.extend(text_data[slice(*command["range"])])
                    text_cursor += 1
                command = image_by_paint.get(p)
                if command is not None:
                    check(
                        not text_by_paint[p]
                        and command == image["commands"][image_cursor]
                        and command["page"] == page
                        and command["owner"] == g["owner"],
                        "actual Figure occurrence consumes only its original image command",
                    )
                    sink.extend(image_data[slice(*command["range"])])
                    sink.extend(b"\n")
                    image_cursor += 1
                paint_cursor += 1
            if inline:
                sink.extend(b"EMC\n")
            sink.extend(m.end_bytes(g))
            sink.extend(b"Q\n")
            ranges.append([start_group, len(sink.bytes)])
            group_data = bytes(sink.bytes[start_group:])
            strict_equal(
                group_data.decode(),
                source["groups"][cursor]["bytes"],
                "complete semantic/artifact group bytes",
            )
            check(
                artifact(root, stage["groups"][cursor]) == group_data,
                "actual original group artifact",
            )
            cursor += 1
        sink.extend(b"Q\n")
        pages.append(
            dict(
                page=page, groups=[group_start, cursor], range=[start, len(sink.bytes)]
            )
        )
        actual = stage["pages"][page]
        strict_equal(
            {k: actual[k] for k in ("index", "groups")},
            dict(index=page, groups=[group_start, cursor]),
            "actual page and group range",
        )
        check(
            artifact(root, actual["bytes"]) == bytes(sink.bytes[start:]),
            "exact local q/Q source page without extra Y flip",
        )
    check(
        cursor == len(groups)
        and text_cursor == len(text["commands"])
        and image_cursor == len(image["commands"])
        and paint_cursor == len(parent["paints"]),
        "every original group, joined paint and command consumed exactly once",
    )
    body_length = len(sink.bytes)
    first = max(probe["font_objects"]["next"], probe["image_objects"]["next"])
    objects = m.encode_anchor_objects(sink, first, bool(anchors))
    strict_equal(
        stage["anchors"], anchors, "actual source-owned invisible extraction anchors"
    )
    strict_equal(
        stage["anchor_objects"],
        objects,
        "exact semantic anchor object identities and ranges",
    )
    check(
        artifact(root, stage["anchor_bytes"]) == bytes(sink.bytes[body_length:]),
        "original semantic anchor objects",
    )
    check(
        stage["byte_length"] == body_length
        and stage["next_object"] == first + len(objects)
        and stage["anchor_font"] == (first if objects else None),
        "marked body length and actual exclusive object cursor",
    )
    hash_probe = dict(
        marked_scopes=parent["marked_scopes"],
        paint_probe=dict(text_commands=parent["text_commands"]),
    )
    fp = m.content_hash(hash_probe, bytes(sink.bytes), pages, ranges, anchors, scratch)
    check(
        fp.hex() == stage["fingerprint"],
        "complete marked content canonical fingerprint",
    )
    return groups, pages, ranges, anchors


def prior_hashes(probe):
    """Reconstruct reported successful canonical folds from checked source data.

    Earlier structure/relation/navigation resource counters remain actual issuer
    observations; this function does not claim their missing allocation trace.
    """
    prior, source = probe["prior_stages"], probe["structural_probe"]
    m, rels = navigation.marked, navigation.relations
    budget = PDFBudget([0, 0, 0, 0])
    adapter = dict(
        marked_content=prior["marked"],
        navigation=dict(flow_fingerprint=prior["syntax_flow_fingerprint"]),
    )
    fp = m.structure_hash(adapter, source["nodes"], source["bindings"], budget)
    check(
        fp.hex() == prior["structure"]["fingerprint"],
        "original syntax/marked/source registry fingerprint chain",
    )
    internal_nodes, headers, related = [], [], []
    output = source["relations"]
    note_by_node = {n["node"]: i for i, n in enumerate(output["notes"])}
    for index, node in enumerate(output["nodes"]):
        rs = len(related)
        related.extend(node["related"])
        internal_nodes.append(
            dict(
                node,
                header_range=[0, 0],
                related_range=[rs, len(related)],
                note_index=note_by_node.get(index),
                numbering=node["list_numbering"],
            )
        )
    # Header storage follows the original table registry and each table's
    # body-cell order. Non-header-bearing nodes keep the original empty0..0;
    # nested table cells need not occur in that storage's node-index order.
    values = m.owner_values(source["wire"]["document"])
    indices = {tuple(n["key"]): i for i, n in enumerate(source["nodes"])}
    for node in source["nodes"]:
        value = values[node["key"][0]]
        if node["key"][1] != "Source" or value.get("kind") != "table":
            continue
        for row in value["body"]:
            for cell in row["cells"]:
                target = internal_nodes[indices[cell["node_id"], "Source"]]
                if target["headers"]:
                    target["header_range"] = [
                        len(headers),
                        len(headers) + len(target["headers"]),
                    ]
                    headers.extend(target["headers"])
    edges = [
        dict(
            edge,
            reference_link=edge["ref_link"],
            reference_label=edge["ref_label"],
            definition_link=edge["def_link"],
            definition_label=edge["def_label"],
        )
        for edge in output["edges"]
    ]
    notes = []
    for index, note in enumerate(output["notes"]):
        per = [i for i, edge in enumerate(edges) if edge["definition"] == index]
        notes.append(dict(note, last=per[-1] if per else None))
    fp = rels.canonical_hash(
        prior["structure"]["fingerprint"],
        internal_nodes,
        output["order"],
        headers,
        related,
        notes,
        edges,
        budget,
    )
    check(
        fp.hex() == prior["relations"]["fingerprint"],
        "exact source Headers/Note relation canonical fold",
    )
    nav = output["navigation"]
    links = []
    for index, link in enumerate(nav["links"]):
        per = [i for i, rect in enumerate(nav["rectangles"]) if rect["link"] == index]
        links.append(dict(link, last=per[-1] if per else None))
    ranges, cursor = [], 0
    for page in range(nav["raw_page_count"]):
        count = nav["page_counts"][page]
        ranges.append([cursor, cursor + count])
        cursor += count
    derived = dict(
        destinations=nav["destinations"],
        links=links,
        rectangles=nav["rectangles"],
        pages=ranges,
        outline=nav["outline"],
    )
    fp = navigation.canonical_hash(
        prior["relations"]["fingerprint"], derived, nav["source_outline"], budget
    )
    check(
        fp.hex() == prior["navigation"]["fingerprint"],
        "original source navigation canonical fold",
    )


def closed_structure(source):
    fields(
        source,
        ("display", "wire", "nodes", "bindings", "groups", "relations"),
        "complete structural PDF integration wire",
    )
    for node in source["nodes"]:
        fields(
            node,
            (
                "key",
                "parent",
                "depth",
                "first_child",
                "next_sibling",
                "first_binding",
                "role",
                "semantic_kind",
                "language",
                "alt",
                "span",
                "cell",
            ),
            "actual source registry node",
        )
    for binding in source["bindings"]:
        fields(
            binding,
            ("node", "group", "page", "mcid", "next"),
            "actual dense MCID binding",
        )
    for group in source["groups"]:
        fields(group, ("owner", "role", "page", "bytes"), "actual marked source group")
    rel = fields(
        source["relations"],
        ("navigation", "nodes", "order", "notes", "edges", "first_demand"),
        "actual original source relations",
    )
    for node in rel["nodes"]:
        fields(
            node,
            (
                "parent",
                "first_child",
                "next",
                "depth",
                "headers",
                "related",
                "list_numbering",
            ),
            "actual related source node",
        )
    for note in rel["notes"]:
        fields(
            note,
            ("node", "link", "label", "painted", "first", "return", "reading", "after"),
            "actual original Note relation",
        )
    for edge in rel["edges"]:
        fields(
            edge,
            (
                "reference",
                "note",
                "definition",
                "source_definition",
                "ref_link",
                "ref_label",
                "def_link",
                "def_label",
                "painted",
                "next",
            ),
            "actual reference edge",
        )
    nav = fields(
        rel["navigation"],
        (
            "assembly",
            "destinations",
            "links",
            "rectangles",
            "page_counts",
            "outline",
            "raw_groups",
            "nonpainting_lines",
            "raw_page_count",
            "source_anchors",
            "inline_anchors",
            "source_outline",
        ),
        "actual source navigation integration",
    )
    schemas = dict(
        destinations=("owner", "kind", "position"),
        links=("node", "target", "first"),
        rectangles=("link", "page", "fragment", "group", "bounds", "next"),
        outline=(
            "destination",
            "parent",
            "first",
            "last",
            "previous",
            "next",
            "descendants",
        ),
        raw_groups=("page", "fragment", "artifact", "paints"),
        nonpainting_lines=(
            "owner",
            "page",
            "fragment",
            "line",
            "breaks",
            "bounds",
            "repeated",
        ),
        inline_anchors=("owner", "page", "fragment", "x", "y", "repeated"),
        source_outline=("id", "parent", "level", "target", "label"),
    )
    for key, schema in schemas.items():
        for row in nav[key]:
            fields(row, schema, "actual navigation " + key)
    fields(
        nav["assembly"],
        (
            "pdf",
            "hash",
            "bytes",
            "catalog",
            "pages",
            "page_names",
            "page_frames",
            "footnote_regions",
            "candidates",
            "references",
            "labels_match",
        ),
        "same actual structural PDF result",
    )


def retained_parent_view(core, probe):
    """Read data only after the genuine retained Navigation kernel is verified.

    No adapted value is passed to the running-region verifier. No automatic
    driver observation or Body owner is manufactured by this projection.
    """
    parent = probe["parent"]
    check(
        parent["wire"] == navigation.SCHEMA
        and parent["observed"]["scope"] == "retained_source_cut"
        and parent["observed"]["parent_scope"] == "retained_source_cut",
        "genuine retained-cut Navigation parent lineage",
    )
    check(
        parent["error"] is None and parent["navigation"] is not None,
        "successful retained Navigation result",
    )
    rel = parent["relation_probe"]
    marked = rel["marked_probe"]
    paint = marked["paint_probe"]
    base = paint["display_probe"]
    wire = marked["source_package"]["wire"]
    pages = []
    for index, page in enumerate(base["pages"]):
        check(
            page["named_index"] is None and page["balance"] is None,
            "this retained fixture has no invented named/balance driver context",
        )
        master = core.selected_master(wire["page_masters"], index)
        check(
            master["header_content"] is None and master["footer_content"] is None,
            "retained six-stream source has no omitted running regions",
        )
        pages.append(
            dict(
                index=index,
                named_page=None,
                selected_master_id=master["master_id"],
                full_body=page["body"],
                declared_note=page["footnote"],
                columns=[c["bounds"] for c in page["columns"]],
                balance_fingerprint=None,
            )
        )
    source = probe["structural_probe"]
    expected_nodes = []
    for node in marked["source_structure"]["nodes"]:
        converted = dict(node)
        converted["alt"] = converted.pop("alternative")
        expected_nodes.append(converted)
    strict_equal(
        source["nodes"],
        expected_nodes,
        "same fully verified original retained source nodes",
    )
    strict_equal(
        source["bindings"],
        marked["source_structure"]["bindings"],
        "same actual retained MCID bindings",
    )
    actual_marked = marked["marked_content"]
    top = probe["marked"]
    for key in (
        "fingerprint",
        "next_object",
        "anchor_font",
        "anchor_bytes",
        "anchor_objects",
        "anchors",
        "groups",
    ):
        strict_equal(
            top[key], actual_marked[key], "same fully verified retained marked " + key
        )
    strict_equal(
        top["byte_length"],
        actual_marked["body_length"],
        "same retained raw body length",
    )
    strict_equal(
        top["pages"],
        [
            dict(index=p["page"], groups=p["groups"], bytes=p["bytes"])
            for p in actual_marked["pages"]
        ],
        "same complete retained local page streams",
    )
    strict_equal(
        probe["prior_stages"]["syntax_flow_fingerprint"],
        marked["navigation"]["flow_fingerprint"],
        "same verified original retained syntax flow issuer",
    )
    return dict(
        case=parent["case"],
        source_canonical_jcs=marked["source_package"]["canonical_jcs"],
        source_package=wire,
        base_display=base,
        limits_canonical_jcs=base["limits_canonical_jcs"],
        limits_fingerprint=base["limits_fingerprint"],
        display_fingerprint=base["fingerprint"],
        pages=pages,
        font_objects=paint["font_objects"],
        image_objects=paint["image_objects"],
        cids=paint["cids"],
        marked_scopes=marked["marked_scopes"],
        observed=dict(maximum_work=parent["observed"]["maximum_work"]),
        downstream=dict(
            constructors=parent["observed"]["constructors"],
            caller_before=None,
            content=actual_marked["charges"],
            structure=marked["source_structure"]["charges"],
            relations=rel["relations"]["charges"],
            navigation=parent["navigation"]["charges"],
        ),
    )


def verify(root, probe, _parent_cache=None):
    fields(
        probe,
        (
            "wire",
            "case",
            "source_canonical_jcs",
            "limits_canonical_jcs",
            "limits_fingerprint",
            "pages",
            "structural_probe",
            "parent",
            "navigation",
            "engine",
            "size_of",
            "observed",
            "assembly",
            "font_objects",
            "image_objects",
            "marked",
            "prior_stages",
        ),
        "closed actual Column PDF wire",
    )
    check(
        probe["wire"] == SCHEMA and type(probe["case"]) is str and probe["case"],
        "actual complete Column PDF successful snapshot",
    )
    import verify_book_v2_column_running_regions as regions

    core = importlib.import_module("verify_book_v2_pdf_assembly")
    old_nav = importlib.import_module("verify_book_v2_navigation_geometry")
    source, parent, prior = (
        probe["structural_probe"],
        probe["parent"],
        probe["prior_stages"],
    )
    strict_equal(probe["case"], parent["case"], "same actual parent callback case")
    # Normal verification always independently replays the full genuine parent.
    # A private self-test cache may later reuse an immutable exact parent only.
    cached_parent = (
        _parent_cache is not None
        and parent is _parent_cache.get("parent")
        and root.resolve() == _parent_cache.get("root")
    )
    actual_parent = parent
    manual = parent["wire"] == navigation.SCHEMA
    if cached_parent:
        previous = dict(_parent_cache["previous"])
        # The original parent is immutable, but the current top-level marked
        # reports can be mutated. Rebind those reports even on the private
        # retained-parent cache path.
        parent = (
            retained_parent_view(core, probe)
            if manual
            else _parent_cache["parent_view"]
        )
    elif manual:
        navigation.verify(root, actual_parent)
        parent = retained_parent_view(core, probe)
        previous = dict(
            regions=0, navigation_prefix=actual_parent["navigation"]["charges"]
        )
    else:
        previous = regions.verify(root, parent)
    inputs = tuple(
        probe[k]
        for k in (
            "pages",
            "structural_probe",
            "marked",
            "font_objects",
            "image_objects",
            "prior_stages",
            "navigation",
            "engine",
            "size_of",
        )
    )
    cached_input = cached_parent and all(
        a is b for a, b in zip(inputs, _parent_cache["inputs"])
    )
    check(
        probe["source_canonical_jcs"] == parent["source_canonical_jcs"],
        "same complete source JCS input bytes",
    )
    if not cached_input:
        closed_structure(source)
        check(
            canonical(source["wire"]) == probe["source_canonical_jcs"],
            "same exact authentic original package JCS",
        )
        strict_equal(
            source["wire"],
            parent["source_package"],
            "unchanged source package and text buffers",
        )
    check(
        probe["limits_canonical_jcs"] == parent["limits_canonical_jcs"]
        and probe["limits_fingerprint"]
        == parent["limits_fingerprint"]
        == sha(probe["limits_canonical_jcs"].encode()),
        "same admitted inclusive resource limits",
    )
    strict_equal(
        source["display"],
        list(bytes.fromhex(parent["display_fingerprint"])),
        "same actual Column display including running regions",
    )
    observed = fields(
        probe["observed"],
        (
            "scope",
            "parent_scope",
            "maximum_work",
            "constructors",
            "caller_before",
            "previous",
            "navigation_prefix",
            "build_after",
            "caller_after",
        )
        + (("source_prefix",) if manual else ()),
        "authentic driver or retained-kernel observations",
    )
    if manual:
        check(
            observed["scope"] == "retained_source_cut"
            and observed["parent_scope"] == "retained_source_cut",
            "retained kernel cannot claim a public PDF driver callback",
        )
        check(
            all(
                observed[k] is None
                for k in (
                    "caller_before",
                    "previous",
                    "navigation_prefix",
                    "caller_after",
                )
            ),
            "retained kernel invents no whole caller or driver observations",
        )
        strict_equal(
            observed["source_prefix"],
            probe["navigation"]["charges"],
            "actual retained Navigation source quartet",
        )
    else:
        check(
            observed["scope"] == "pdf_driver"
            and observed["parent_scope"] == "navigation_driver",
            "only actual successful typed Column PDF driver callback",
        )
    strict_equal(
        observed["maximum_work"],
        parent["observed"]["maximum_work"],
        "same accepted command Work ceiling",
    )
    ctors = observed["constructors"]
    check(
        type(ctors) is list and len(ctors) == 14,
        "all fourteen genuine constructor snapshots",
    )
    strict_equal(
        ctors[:13],
        parent["downstream"]["constructors"],
        "unchanged thirteen genuine Navigation ancestor constructors",
    )
    strict_equal(
        observed["caller_before"],
        observed["previous"],
        "genuine previous whole caller quartet",
    )
    strict_equal(
        observed["previous"],
        parent["downstream"]["caller_before"],
        "same whole command prior across typed phase bridges",
    )
    if not manual:
        strict_equal(
            observed["navigation_prefix"],
            probe["navigation"]["charges"],
            "genuine completed Navigation callback before constructor14",
        )
    strict_equal(
        previous["navigation_prefix"],
        probe["navigation"]["charges"],
        "same authenticated parent Navigation quartet",
    )
    strict_equal(
        probe["navigation"],
        prior["navigation"],
        "same navigation result identity facts",
    )
    fields(
        prior,
        (
            "syntax_flow_fingerprint",
            "scopes",
            "marked",
            "structure",
            "relations",
            "navigation",
        ),
        "complete genuine canonical chain linkage",
    )
    for key, earlier in (
        ("scopes", parent["marked_scopes"]),
        (
            "marked",
            dict(
                fingerprint=probe["marked"]["fingerprint"],
                charges=parent["downstream"]["content"],
            ),
        ),
        (
            "structure",
            dict(
                charges=parent["downstream"]["structure"],
                fingerprint=prior["structure"]["fingerprint"],
            ),
        ),
        (
            "relations",
            dict(
                charges=parent["downstream"]["relations"],
                fingerprint=prior["relations"]["fingerprint"],
            ),
        ),
    ):
        fields(prior[key], ("fingerprint", "charges"), "actual canonical stage " + key)
        strict_equal(
            prior[key]["fingerprint"],
            earlier["fingerprint"],
            "same genuine " + key + " fingerprint",
        )
        strict_equal(
            prior[key]["charges"],
            earlier["charges"],
            "same genuine " + key + " quartet",
        )
    fields(
        probe["engine"],
        ("name", "version", "rust_version", "git_commit"),
        "compiled issuer identity facts",
    )
    check(
        probe["engine"]["name"] == "typaxis" and probe["engine"]["version"] == "0.1.0",
        "actual serialized engine name/version",
    )
    check(
        all(
            probe["engine"][key] is None or type(probe["engine"][key]) is str
            for key in ("rust_version", "git_commit")
        ),
        "issuer-only runtime metadata type",
    )
    size = fields(
        probe["size_of"],
        (
            "selected_page_master",
            "object_range",
            "page_reference",
            "option_usize",
            "usize",
        ),
        "actual assembly reservation sizes",
    )
    strict_equal(
        size,
        dict(
            usize=8,
            object_range=16,
            option_usize=16,
            page_reference=20,
            selected_page_master=32,
        ),
        "actual target ABI and readonly private reservation units",
    )
    # Only the private corruption runner can provide this cache. Its key uses
    # the exact immutable input objects verified by a full normal call. Every
    # changed source or encoder input takes the complete normal path. The
    # complete original JSON and all its artifact bytes are checked again at
    # the end of each matrix. There is no accepted JSON cache/receipt field.
    stage = fields(
        probe["assembly"],
        (
            "pdf",
            "fingerprint",
            "charges",
            "first",
            "next",
            "objects",
            "catalog",
            "pages",
            "candidates",
            "references",
            "labels_match",
        ),
        "full actual PDF assembly",
    )
    earlier = source["relations"]["navigation"]["assembly"]
    for key in ("catalog", "pages", "candidates", "references", "labels_match"):
        strict_equal(
            stage[key], earlier[key], "same actual structural PDF field " + key
        )
    if not cached_input:
        verify_frames(core, dict(probe, parent=parent))
    # The old cores independently reconstruct original source topology,
    # Headers, first-reference Note forest, links and raw physical geometry.
    if not cached_input:
        old_nav.verify(source)
        expected_nodes = importlib.import_module(
            "verify_book_v2_source_structure"
        ).source_nodes(source["wire"]["document"])
        for node, expected in zip(source["nodes"], expected_nodes):
            strict_equal(
                node["language"],
                navigation.marked.canonical_language(expected["language"]),
                "exact canonical original source language",
            )
    fields(
        probe["marked"],
        (
            "fingerprint",
            "byte_length",
            "next_object",
            "anchor_font",
            "anchor_bytes",
            "anchor_objects",
            "pages",
            "groups",
            "anchors",
        ),
        "complete actual source marked materialization",
    )
    fields(
        probe["font_objects"],
        ("fingerprint", "bytes", "first", "next", "objects", "resources"),
        "all actual font objects",
    )
    strict_equal(
        {k: probe["font_objects"][k] for k in parent["font_objects"] if k != "charges"},
        {k: v for k, v in parent["font_objects"].items() if k != "charges"},
        "same genuine font object chain",
    )
    font_resources = []
    for index, font in enumerate(parent["cids"]["fonts"]):
        object_id = next(
            o["id"]
            for o in parent["font_objects"]["objects"]
            if o["font"] == index and o["role"] == "Type0"
        )
        font_resources.append(
            dict(index=index, instance=font["instance"], object=object_id)
        )
    strict_equal(
        probe["font_objects"]["resources"],
        font_resources,
        "all original typed font resource names and object identities",
    )
    fields(
        probe["image_objects"],
        ("fingerprint", "bytes", "first", "next", "objects", "resources"),
        "all actual image objects",
    )
    image_parent = {k: v for k, v in parent["image_objects"].items() if k != "charges"}
    image_parent["resources"] = [
        dict(index=i, object=o) for i, o in enumerate(image_parent["resources"])
    ]
    strict_equal(
        probe["image_objects"],
        image_parent,
        "same fully verified vector/raster image object chain",
    )
    if not cached_input:
        if not manual:
            verify_marked_bytes(root, probe, regions)
        prior_hashes(probe)
    marked_pages = [artifact(root, p["bytes"]) for p in probe["marked"]["pages"]]
    payload = dict(
        font_objects=artifact(root, probe["font_objects"]["bytes"]),
        image_objects=artifact(root, probe["image_objects"]["bytes"]),
        anchor_bytes=artifact(root, probe["marked"]["anchor_bytes"]),
        marked_pages=marked_pages,
    )
    closed = dict(probe, _payload=payload)
    if cached_input:
        result = dict(_parent_cache["result"])
        result["budget"] = PDFBudget(_parent_cache["result"]["budget"].triple4())
        expected_ctor = list(probe["navigation"]["charges"])
        expected_ctor[0] += 1
        strict_equal(ctors[13], expected_ctor, "genuine counted Assembly constructor14")
    else:
        result = assembly_replay(core, root, closed)
    if _parent_cache is not None and not _parent_cache.get("result"):
        _parent_cache.update(
            root=root.resolve(),
            parent=actual_parent,
            parent_view=parent,
            previous=previous,
            inputs=inputs,
            result=result,
        )
    for key, value in (
        ("first", result["plan"]["first"]),
        ("next", result["plan"]["next"]),
        ("catalog", result["plan"]["first"]),
        ("objects", result["objects"]),
        ("pages", [result["plan"]["page"] + 2 * i for i in range(len(probe["pages"]))]),
    ):
        strict_equal(stage[key], value, "independent complete PDF " + key)
    check(
        artifact(root, stage["pdf"]) == result["data"],
        "every independently encoded final PDF byte",
    )
    check(
        stage["fingerprint"] == result["fingerprint"],
        "exact final source-bound PDF canonical fingerprint",
    )
    result["budget"].matches4(stage)
    strict_equal(
        observed["build_after"],
        stage["charges"],
        "genuine first Assembly build quartet",
    )
    if not manual:
        strict_equal(
            observed["caller_after"],
            stage["charges"],
            "typed assembly signed output reaches actual whole caller teardown",
        )
    check(
        earlier["pdf"] == stage["pdf"]["file"]
        and earlier["hash"] == list(bytes.fromhex(stage["pdf"]["sha256"]))
        and earlier["bytes"] == len(result["data"]),
        "same actual complete PDF artifact",
    )
    if not cached_input:
        core.verify_xref(result["data"])
        from pypdf import PdfReader

        core.verify(PdfReader(io.BytesIO(result["data"]), strict=True), source)
    caps = json.loads(probe["limits_canonical_jcs"])["base"]
    for quartet in [
        *ctors,
        *[
            prior[k]["charges"]
            for k in ("scopes", "marked", "structure", "relations", "navigation")
        ],
        stage["charges"],
    ]:
        r, s, o, w = PDFBudget(quartet).triple4()
        check(
            r <= caps["max_fragments"]
            and s <= caps["max_spool_bytes"]
            and o <= caps["max_output_bytes"]
            and w <= observed["maximum_work"],
            "all accepted upstream peaks and net final quartet fit inclusive caps",
        )
    return dict(
        pages=len(probe["pages"]),
        columns=sum(len(p["columns"]) for p in probe["pages"]),
        running_regions=previous["regions"],
        source_nodes=len(source["nodes"]),
        bindings=len(source["bindings"]),
        annotations=len(source["relations"]["navigation"]["rectangles"]),
        pdf_bytes=len(result["data"]),
        pdf_objects=sum(v is not None for v in result["objects"]),
        replacement_credit=result["replacement_credit"],
        signed_output_delta=len(result["data"]) - result["replacement_credit"],
    )


REJECTION_ERRORS = (
    AssertionError,
    KeyError,
    IndexError,
    ValueError,
    TypeError,
    UnicodeError,
    StopIteration,
    OverflowError,
)


def path_copy(value, path, replacement, delete=False):
    if not path:
        check(not delete, "cannot delete the root probe")
        return replacement
    key, *tail = path
    result = dict(value) if isinstance(value, dict) else list(value)
    if tail:
        result[key] = path_copy(value[key], tail, replacement, delete)
    elif delete:
        del result[key]
    else:
        result[key] = replacement
    return result


def tamper_cases(probe):
    """Enumerate each new Assembly field; inherited matrices remain separate.

    The full parent receives fresh normal replay. Its historical exhaustive
    matrices are not counted again as new PDF mutations. Original source and
    PDF graph controls below exercise this stage's additional integration.
    """

    def leaves(value, path):
        if isinstance(value, dict):
            for key, child in value.items():
                yield from leaves(child, path + [key])
        elif isinstance(value, list):
            for index, child in enumerate(value):
                yield from leaves(child, path + [index])
        else:
            yield path, value

    def alternative(value):
        if type(value) is bool:
            return not value
        if type(value) is int:
            return value + 1
        if value is None:
            return 0
        if type(value) is str:
            if re.fullmatch(r"[0-9a-f]{64}", value):
                return ("1" if value[0] == "0" else "0") + value[1:]
            return value + "!"
        raise AssertionError("closed new PDF scalar")

    for branch in (
        "pages",
        "navigation",
        "engine",
        "size_of",
        "observed",
        "assembly",
        "font_objects",
        "image_objects",
        "marked",
        "prior_stages",
    ):
        for path, value in leaves(probe[branch], [branch]):
            # rust/git are retained issuer facts. Their presence/type, but not
            # their independent truth, is checked. Do not claim arbitrary
            # same-type changes to unused PDF metadata are detectable.
            if path in (["engine", "rust_version"], ["engine", "git_commit"]):
                continue
            yield "/".join(map(str, path)), path_copy(probe, path, alternative(value))
        if isinstance(probe[branch], dict):
            for key in probe[branch]:
                yield "missing/" + branch + "/" + key, path_copy(
                    probe, [branch, key], None, True
                )
        else:
            yield "incomplete/" + branch, path_copy(probe, [branch], probe[branch][:-1])
    for branch in (
        "wire",
        "case",
        "source_canonical_jcs",
        "limits_canonical_jcs",
        "limits_fingerprint",
    ):
        yield "root/" + branch, path_copy(probe, [branch], probe[branch] + "!")
    for key in ("pages", "objects", "candidates", "references"):
        value = probe["assembly"][key]
        yield "assembly/incomplete/" + key, path_copy(
            probe, ["assembly", key], value[:-1] if value else [None]
        )
    for index, page in enumerate(probe["pages"]):
        yield "column-order/" + str(index), path_copy(
            probe, ["pages", index, "columns"], list(reversed(page["columns"]))
        )
    if probe["observed"]["scope"] == "retained_source_cut":
        value = path_copy(probe, ["observed", "scope"], "pdf_driver")
        value = path_copy(value, ["observed", "parent_scope"], "navigation_driver")
        for key, replacement in (
            ("caller_before", [0, 0, 0, 0]),
            ("previous", [0, 0, 0, 0]),
            ("navigation_prefix", probe["navigation"]["charges"]),
            ("caller_after", probe["assembly"]["charges"]),
        ):
            value = path_copy(value, ["observed", key], replacement)
        yield "joint/retained-kernel-forged-driver", value
    # Couple reported accounting to a tempting alternate gross-output model.
    bad = list(probe["assembly"]["charges"])
    bad[2] += (
        probe["marked"]["byte_length"]
        + probe["font_objects"]["bytes"]["length"]
        + probe["image_objects"]["bytes"]["length"]
    )
    value = path_copy(probe, ["assembly", "charges"], bad)
    value = path_copy(value, ["observed", "build_after"], bad)
    value = path_copy(value, ["observed", "caller_after"], bad)
    yield "joint/gross-output-credit-revived", value
    # Raw first-column translation must be rejected even when duplicated top
    # frame reports agree. The original authored master remains unchanged.
    for index, page in enumerate(probe["pages"][:1]):
        cols = copy.deepcopy(page["columns"])
        cols[0][0] += 1
        value = path_copy(probe, ["pages", index, "columns"], cols)
        if probe["observed"]["scope"] == "retained_source_cut":
            value = path_copy(
                value,
                [
                    "parent",
                    "relation_probe",
                    "marked_probe",
                    "paint_probe",
                    "display_probe",
                    "pages",
                    index,
                    "columns",
                    0,
                    "bounds",
                ],
                cols[0],
            )
        else:
            value = path_copy(value, ["parent", "pages", index, "columns"], cols)
        yield "joint/physical-column-origin", value


def reported_assembly_fingerprint(probe, data):
    budget = PDFBudget([0, 0, 0, 0])
    fp = budget.fold(bytes.fromhex(probe["navigation"]["fingerprint"]), ALGORITHM)
    for first in range(0, len(data), 96):
        fp = budget.fold(fp, data[first : first + 96])
    for ref in probe["assembly"]["references"]:
        value = b"".join(
            integer(ref[key], 32, False).to_bytes(4, "big")
            for key in ("owner", "candidate")
        )
        value += integer(ref["target"] or 0, 32, False).to_bytes(4, "big") + bytes(
            [ref["placed"]]
        )
        fp = budget.fold(fp, value)
    return fp.hex()


def artifact_controls(root, probe, cache):
    data = artifact(root, probe["assembly"]["pdf"])
    actions = [("header", lambda b: b.__setitem__(1, ord("X")))]
    transform = data.find(b"1 0 0 -1 0 ")
    if transform >= 0:
        actions.append(
            (
                "page-y-flip",
                lambda b: b.__setitem__(slice(transform + 6, transform + 8), b" 1"),
            )
        )
    xref = int(re.search(rb"startxref\n([0-9]+)\n%%EOF\n$", data)[1])
    actions.append(("xref-offset", lambda b: b.__setitem__(xref + 2, ord("X"))))
    count = 0
    for label, change in actions:
        changed = bytearray(data)
        change(changed)
        value = bytes(changed)
        descriptor = dict(
            file="selftest-pdf-" + sha(value) + ".bin",
            sha256=sha(value),
            length=len(value),
        )
        path = root / descriptor["file"]
        check(not path.exists(), "private PDF corruption artifact is new")
        path.write_bytes(value)
        try:
            bad = path_copy(probe, ["assembly", "pdf"], descriptor)
            bad = path_copy(
                bad,
                ["assembly", "fingerprint"],
                reported_assembly_fingerprint(bad, value),
            )
            # Reseal the duplicated artifact reports, preserving original
            # source/master facts. A checksum-only verifier would accept this.
            for key, replacement in (
                ("pdf", descriptor["file"]),
                ("hash", list(bytes.fromhex(descriptor["sha256"]))),
                ("bytes", len(value)),
            ):
                bad = path_copy(
                    bad,
                    ["structural_probe", "relations", "navigation", "assembly", key],
                    replacement,
                )
            rejected = False
            try:
                verify(root, bad, cache)
            except REJECTION_ERRORS:
                rejected = True
            check(
                rejected,
                "SHA-and-fingerprint-resealed PDF corruption rejected: " + label,
            )
            count += 1
        finally:
            path.unlink()
    return count


def original_artifacts(root, probe):
    values = {}

    def visit(value):
        if isinstance(value, dict):
            if set(value) == {"file", "sha256", "length"}:
                data = artifact(root, value)
                values[value["file"]] = sha(data)
            else:
                for child in value.values():
                    visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(probe)
    return values


def self_test(root, probe):
    original = sha(canonical(probe).encode())
    artifacts = original_artifacts(root, probe)
    cache = {}
    verify(root, probe, cache)
    count = 0
    for label, bad in tamper_cases(probe):
        # A path copy cannot alter the shared original. All changed upstream
        # objects leave the identity cache and take a full independent replay.
        rejected = False
        try:
            verify(root, bad, cache)
        except REJECTION_ERRORS:
            rejected = True
        check(rejected, "new Column PDF corruption accepted: " + label)
        count += 1
    count += artifact_controls(root, probe, cache)
    core = importlib.import_module("verify_book_v2_pdf_assembly")
    semantic = core.tamper_checks(
        artifact(root, probe["assembly"]["pdf"]), probe["structural_probe"]
    )
    check(
        sha(canonical(probe).encode()) == original,
        "complete original immutable JSON unchanged after every mutation",
    )
    strict_equal(
        original_artifacts(root, probe),
        artifacts,
        "every complete original artifact SHA unchanged after matrix",
    )
    return count, semantic


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path, nargs="+")
    parser.add_argument("--pypdf-site", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    environment = configure_pdf_site(args.pypdf_site)
    # Load the unchanged parser cores before recording their source snapshot.
    # The Body-only command-line master guard is deliberately never invoked.
    importlib.import_module("verify_book_v2_column_running_regions")
    importlib.import_module("verify_book_v2_pdf_assembly")
    importlib.import_module("verify_book_v2_navigation_geometry")

    def inherited_snapshot():
        return {
            str(
                Path(module.__file__)
                .resolve()
                .relative_to(Path(__file__).resolve().parent)
            ): sha(Path(module.__file__).read_bytes())
            for name, module in sorted(sys.modules.items())
            if name.startswith("verify_book_v2_")
            and getattr(module, "__file__", None)
            and Path(module.__file__).resolve().parent
            == Path(__file__).resolve().parent
        }

    inherited = inherited_snapshot()
    checker_sha = sha(Path(__file__).read_bytes())
    files = sorted(
        {
            p.resolve()
            for directory in args.directory
            for p in directory.rglob("*.json")
            if json.loads(p.read_text()).get("wire") == SCHEMA
        },
        key=lambda p: (p.name, str(p)),
    )
    check(files, "actual complete Column PDF probe inputs")
    totals, cases, scopes, negatives, semantic_negatives = (
        Counter(),
        [],
        Counter(),
        0,
        0,
    )
    for path in files:
        raw_probe = path.read_bytes()
        raw_probe_sha = sha(raw_probe)
        probe = json.loads(raw_probe)
        result = verify(path.parent, probe)
        totals.update(result)
        check(probe["case"] not in cases, "unique actual PDF callback case")
        cases.append(probe["case"])
        scopes[probe["observed"]["scope"]] += 1
        print(
            "VERIFY " + path.name + " " + json.dumps(result, sort_keys=True), flush=True
        )
        if args.self_test:
            own, semantic = self_test(path.parent, probe)
            negatives += own
            semantic_negatives += semantic
            print(
                "SELFTEST "
                + path.name
                + " "
                + str(own)
                + " PDF-SEMANTIC "
                + str(semantic),
                flush=True,
            )
        check(
            sha(path.read_bytes()) == raw_probe_sha,
            "complete original raw probe file SHA unchanged after verification/matrix",
        )
    check(
        configure_pdf_site(args.pypdf_site) == environment,
        "unchanged explicit pypdf static package start/end",
    )
    strict_equal(
        inherited_snapshot(),
        inherited,
        "every inherited checker source SHA unchanged start/end",
    )
    check(
        sha(Path(__file__).read_bytes()) == checker_sha,
        "new checker source SHA unchanged start/end",
    )
    print(
        json.dumps(
            dict(
                schema="typaxis.book-2-column-pdf-verification/1",
                probes=len(files),
                cases=cases,
                scopes=dict(scopes),
                totals=dict(totals),
                negative_tests=negatives,
                pdf_semantic_negative_tests=semantic_negatives,
                checker_sha256=checker_sha,
                inherited_sources=inherited,
                pypdf=environment,
                issuer_only_engine_fields=["rust_version", "git_commit"],
                scope="Independent original source/Headers/Note/navigation/PDF semantics, original Column frames and exact successful Assembly bytes/counters/canonical hash. Prior structure/relation/navigation quartets and syntax-flow fingerprint remain genuine issuer prefixes; source preparation/shaping, failed attempts/retries, rust/git metadata, PDF/UA and whole-book acceptance are not independently replayed.",
            ),
            sort_keys=True,
        ),
        flush=True,
    )


if __name__ == "__main__":
    main()
