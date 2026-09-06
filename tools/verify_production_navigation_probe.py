#!/usr/bin/env python3
"""Parse selected-navigation diagnostic PDFs independently of Typaxis.

The Rust probe asserts source ownership, line/page counts and actual fragment
positions, then exports those selected receipts as the PDF serialization oracle.
This verifier does not issue public-build, PDF/UA or full-book acceptance receipts.
"""
from __future__ import annotations
import argparse
import json
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ArrayObject, FloatObject, NameObject, NumberObject, TextStringObject


class Failure(ValueError):
    pass


def require(ok, message):
    if not ok:
        raise Failure(message)


def refid(obj):
    ref = getattr(obj, "indirect_reference", obj)
    require(hasattr(ref, "idnum"), "expected indirect reference")
    return ref.idnum


def near(actual, expected):
    return abs(float(actual) - expected) <= 0.00001


def annotations(reader):
    return [(i, ref, ref.get_object()) for i, page in enumerate(reader.pages)
            for ref in page.get("/Annots", [])]


def verify(reader, expected):
    pages = reader.pages
    require(len(pages) == expected["pages"], "page count")
    catalog = reader.trailer["/Root"]
    names = catalog["/Names"]["/Dests"]["/Names"] if expected["destinations"] else []
    if not expected["destinations"]:
        require("/Names" not in catalog, "unexpected destination tree")
    require(len(names) == 2 * len(expected["destinations"]), "destination cardinality")
    keys = [str(names[i]) for i in range(0, len(names), 2)]
    require(keys == sorted(keys, key=lambda k: k.encode("utf-16-be")), "name-tree byte ordering")
    require(len(set(keys)) == len(keys), "duplicate destination")
    destinations = dict(zip(keys, names[1::2]))
    height = expected["page_height_raw"] / 65536
    for dest in expected["destinations"]:
        require(dest["name"] in destinations, "missing destination")
        value = destinations[dest["name"]]
        require(len(value) == 5 and value[1] == "/XYZ", "destination view")
        require(refid(value[0]) == refid(pages[dest["page"]]), "destination page")
        require(near(value[2], dest["x_raw"] / 65536), "destination x")
        require(near(value[3], height - dest["y_raw"] / 65536), "destination y")
    root = catalog["/StructTreeRoot"]
    nums = root["/ParentTree"]["/Nums"]
    require(list(nums[::2]) == list(range(len(pages) + len(expected["links"]))), "ParentTree keys")
    require(root["/ParentTreeNextKey"] == len(pages) + len(expected["links"]), "ParentTreeNextKey")
    parents = dict(zip(nums[::2], nums[1::2]))
    for i, page in enumerate(pages):
        require(page["/StructParents"] == i and isinstance(parents[i], list), "page MCID parent array")
    actual = annotations(reader)
    require(len(actual) == len(expected["links"]), "annotation cardinality")
    require(len({refid(ref) for _, ref, _ in actual}) == len(actual), "duplicate annotation reference")
    objrs = []
    seen = set()

    def walk(ref):
        node = ref.get_object()
        identity = refid(ref)
        require(identity not in seen, "structure cycle or duplicate child")
        seen.add(identity)
        kids = node.get("/K", [])
        if not isinstance(kids, list):
            kids = [kids]
        for kid in kids:
            obj = kid.get_object()
            if obj.get("/Type") == "/StructElem":
                require(refid(obj.raw_get("/P")) == identity, "structure parent backlink")
                walk(kid)
            elif obj.get("/Type") == "/OBJR":
                require(node["/S"] == "/Link", "OBJR source role")
                objrs.append((identity, refid(obj.raw_get("/Pg")), refid(obj.raw_get("/Obj"))))
            else:
                require(obj.get("/Type") == "/MCR", "unexpected structure child")

    for child in root["/K"]:
        walk(child)
    require(len(objrs) == len(actual), "OBJR cardinality")
    for (page, ref, annot), link in zip(actual, expected["links"]):
        require(page == link["page"], "annotation page order")
        require(annot["/Type"] == "/Annot" and annot["/Subtype"] == "/Link", "annotation kind")
        require(refid(annot.raw_get("/P")) == refid(pages[page]), "annotation page backlink")
        if "uri" in link:
            require("/Dest" not in annot and annot["/A"]["/S"] == "/URI", "URI action kind")
            require(str(annot["/A"]["/URI"]) == link["uri"], "URI action bytes")
        else:
            require("/A" not in annot and annot["/Dest"] == link["destination"] and annot["/Dest"] in destinations, "link target")
        x, y, w, h = [v / 65536 for v in link["rect_raw"]]
        rect = [x, height - y - h, x + w, height - y]
        require(len(annot["/Rect"]) == 4 and all(near(a, b) for a, b in zip(annot["/Rect"], rect)), "annotation rectangle")
        require(annot["/Border"] == [0, 0, 0] and annot["/F"] == 4, "annotation appearance flags")
        require(annot["/Contents"] == link["contents"] and bool(link["contents"].strip()), "annotation accessible name")
        key = link["struct_parent"]
        require(annot["/StructParent"] == key, "annotation StructParent")
        require(refid(parents[key]) == link["structure_object"], "annotation ParentTree owner")
        require(parents[key].get_object()["/S"] == "/Link", "annotation structure role")
        require(objrs.count((link["structure_object"], refid(pages[page]), refid(ref))) == 1, "annotation OBJR binding")
    entries = expected["outline"]
    if not entries:
        require("/Outlines" not in catalog, "unexpected outlines")
        return
    outline = catalog["/Outlines"]
    require(outline["/Type"] == "/Outlines" and outline["/Count"] == len(entries), "outline root")
    visited = set()
    children = {}
    for entry in entries:
        children.setdefault(entry["parent"], []).append(entry)

    def descendants(index):
        return sum(1 + descendants(c["id"]) for c in children.get(index, []))

    def siblings(parent, parent_id):
        wanted = children.get(parent_id, [])
        if not wanted:
            require("/First" not in parent and "/Last" not in parent, "unexpected outline children")
            return
        current = parent.raw_get("/First")
        previous = None
        for entry in wanted:
            identity = refid(current)
            require(identity not in visited, "outline cycle")
            visited.add(identity)
            node = current.get_object()
            require(refid(node.raw_get("/Parent")) == refid(parent), "outline parent")
            require(node["/Title"] == entry["title"] and node["/Dest"] == entry["destination"], "outline source identity")
            require(node["/Dest"] in destinations, "outline destination")
            require((refid(node.raw_get("/Prev")) if "/Prev" in node else None) == previous, "outline previous")
            count = descendants(entry["id"])
            require(node.get("/Count", 0) == count, "outline descendant count")
            siblings(node, entry["id"])
            previous = identity
            current = node.raw_get("/Next") if "/Next" in node else None
        require(current is None and previous == refid(parent.raw_get("/Last")), "outline last/next")

    siblings(outline, None)
    require(len(visited) == len(entries), "outline coverage")


def negative_mutations(reader):
    page, ref, annotation = annotations(reader)[0]
    root = reader.trailer["/Root"]["/StructTreeRoot"]
    catalog = reader.trailer["/Root"]
    names = catalog["/Names"]["/Dests"]["/Names"] if "/Names" in catalog else []
    key = int(annotation["/StructParent"])
    parents = root["/ParentTree"]["/Nums"]
    structure = parents[key * 2 + 1].get_object()
    mutations = {
        "missing-annotation": lambda: reader.pages[page].__setitem__(NameObject("/Annots"), ArrayObject()),
        "duplicate-annotation": lambda: reader.pages[page]["/Annots"].append(ref),
        "shifted-rectangle": lambda: annotation["/Rect"].__setitem__(0, FloatObject(float(annotation["/Rect"][0]) + 1)),
        "wrong-link-target": lambda: annotation.__setitem__(NameObject("/Dest"), TextStringObject("absent")),
        "changed-accessible-name": lambda: annotation.__setitem__(NameObject("/Contents"), TextStringObject("wrong source name")),
        "page-key-collision": lambda: annotation.__setitem__(NameObject("/StructParent"), NumberObject(0)),
        "missing-objr": lambda: structure.__setitem__(NameObject("/K"), ArrayObject(k for k in structure["/K"] if k.get_object().get("/Type") != "/OBJR")),
        "wrong-parent-tree-owner": lambda: parents.__setitem__(key * 2 + 1, reader.pages[0].indirect_reference),
    }
    if names:
        mutations["shifted-destination"] = lambda: names[1].__setitem__(2, FloatObject(float(names[1][2]) + 1))
    if "/A" in annotation:
        mutations["changed-uri"] = lambda: annotation["/A"].__setitem__(NameObject("/URI"), TextStringObject("https://example.invalid/"))
    return mutations


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--probe-root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    observed = {"scope": "selected-navigation-diagnostic-probe", "public_build": False, "full_book": False, "cases": []}
    for name in ["vmb-navigation", "multiline-navigation", "external-navigation"]:
        path = args.probe_root / f"{name}.pdf"
        expected = json.loads((args.probe_root / f"{name}.expected.json").read_text())
        verify(PdfReader(path, strict=True), expected)
        rejected = []
        for mutation in negative_mutations(PdfReader(path, strict=True)):
            reader = PdfReader(path, strict=True)
            negative_mutations(reader)[mutation]()
            try:
                verify(reader, expected)
            except Failure as exc:
                rejected.append({"mutation": mutation, "reason": str(exc)})
            else:
                raise Failure(f"accepted mutation {mutation}")
        if expected["outline"]:
            reader = PdfReader(path, strict=True)
            outline = reader.trailer["/Root"]["/Outlines"]
            outline["/First"][NameObject("/Next")] = outline.raw_get("/First")
            try:
                verify(reader, expected)
            except Failure as exc:
                rejected.append({"mutation": "outline-cycle", "reason": str(exc)})
            else:
                raise Failure("accepted outline cycle")
        observed["cases"].append({"name": name, "passed": True, "links": len(expected["links"]), "destinations": len(expected["destinations"]), "negative_probes": rejected})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(observed, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"cases": len(observed["cases"]), "negative_probes_rejected": sum(len(c["negative_probes"]) for c in observed["cases"]), "scope": observed["scope"]}))


if __name__ == "__main__":
    main()
