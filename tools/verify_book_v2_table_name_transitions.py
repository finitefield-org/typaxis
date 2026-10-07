#!/usr/bin/env python3
"""Check named table-transition driver PDFs against closed original fixtures."""
import argparse
import copy
import hashlib
import json
import re
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ContentStream
from verify_book_v2_definition_table_pdf import check_positions
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_pdf_assembly import selected_master, source_page_owner_names

U = 65536
MODES = {"flat", "unequal", "nested", "nested-scopes", "rows", "header", "span", "caption",
         "empty-child", "break", "return", "unnamed", "width"}


def artifact_glyphs(reader):
    pages = []
    for page in reader.pages:
        stack, glyphs = [], []
        matrix = cmap = None
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op in (b'BDC', b'BMC'):
                artifact = args[0] == '/Artifact'
                if artifact:
                    assert op == b'BDC' and args[1] == {'/Type': '/Pagination', '/Subtype': '/Header'}
                stack.append(artifact)
            elif op == b'EMC':
                stack.pop()
            elif op == b'Tf':
                font = page['/Resources']['/Font'][args[0]].get_object()
                pairs = re.findall(rb'<([0-9A-F]+)> <([0-9A-F]+)>', font['/ToUnicode'].get_data().split(b'beginbfchar', 1)[1])
                cmap = {int(a,16): bytes.fromhex(b.decode()).decode('utf-16-be') for a,b in pairs}
            elif op == b'Tm':
                matrix = [float(v) for v in args]
            elif op == b'Tj' and any(stack):
                raw = args[0].original_bytes if hasattr(args[0], 'original_bytes') else bytes(args[0])
                assert len(raw) == 2 and matrix[:4] == [1,0,0,-1]
                glyphs.append([cmap[int.from_bytes(raw,'big')],matrix[4],matrix[5],True])
            elif op == b'TJ':
                raise AssertionError('fixture requires explicit glyph matrices')
        assert not stack
        pages.append(glyphs)
    return pages


def expectations(wire, text):
    table, following = wire["document"]["blocks"]
    mode, = [c.removeprefix("transition-") for c in table["classes"] if c.startswith("transition-")]
    assert mode in MODES and not wire["document"]["footnotes"]
    japanese = not text.isascii()
    assert text == (("左側右側左側右側" if mode == "width" else "左右") if japanese
                    else {"nested": "Res", "nested-scopes": "Res", "width": "Result Result"}.get(mode, "Result"))
    owners = source_page_owner_names(wire)
    original = []
    paragraph_owners, breaks = [], []

    def visit(value):
        if isinstance(value, list):
            for child in value:
                visit(child)
        elif isinstance(value, dict):
            if "node_id" in value:
                original.append(value["node_id"])
            if value.get("kind") == "paragraph":
                paragraph_owners.append(value["node_id"])
                child, = value["children"]
                assert child["kind"] == "text"
                assert child["text_span"]["start_byte"] == 0
                assert child["text_span"]["end_byte"] == len(text.encode())
            if value.get("kind") == "page_break":
                breaks.append(value["node_id"])
            for field in ("blocks", "children", "caption", "head", "body", "cells", "footnotes"):
                if field in value:
                    visit(value[field])

    visit(wire["document"])
    assert original == list(range(len(original))), "original node order"
    assert owners[following["node_id"]] is None
    assert len(paragraph_owners) == {"nested":17,"nested-scopes":17,"header":7,"caption":10}.get(mode,9)
    assert len(breaks) == (2 if mode == "break" else 0)
    assert table["columns"] == [{"kind": "fraction", "weight": 1}] * 2
    rules = {r["style_id"]: r for r in wire["style_sheet"]["rules"]}
    values = {d["name"]: d["value"] for d in rules["paragraph-text"]["declarations"]}
    assert values["font_size"]["value"] == 12 * U
    assert values["line_height"]["value"] == 16 * U
    ascent = round((1151 if japanese else 800) * 12 * U / 1000)
    descent = round((286 if japanese else 200) * 12 * U / 1000)
    line = max(16 * U, ascent + descent)
    baseline = ascent + (line - ascent - descent) // 2
    columns = 4 if mode in ("nested", "nested-scopes") else 2
    phases = [("appendix", [2] * columns), ("short", [2] * columns)]
    if mode == "unequal":
        phases = [("appendix", [1, 3]), ("short", [3, 1])]
    elif mode == "header":
        assert len(table["head"]) == 1
        phases = [("appendix", [1, 1]), ("short", [2, 2])]
    elif mode == "caption":
        assert len(table["caption"]) == 1
        phases = [("appendix", [1]), ("short", [4, 4])]
    elif mode == "empty-child":
        phases = [("short", [4, 4]), ("appendix", [0, 0])]
    elif mode == "return":
        phases = [("appendix", [1, 1]), ("short", [2, 2]), ("appendix", [1, 1])]
    elif mode == "unnamed":
        phases = [("appendix", [2, 2]), (None, [2, 2])]
    # These fixtures use whole repeated paragraphs, pinned font metrics, and
    # the already verified source line partitions of the width family. This
    # oracle does not use captured line positions or table continuation records.
    names, positions, boxes, trims = [], [], [], []

    def frame(name):
        master = selected_master(wire["page_masters"], len(names), name)
        body = master["body"]
        return master, body

    def append(name, wanted):
        master, _ = frame(name)
        names.append(name)
        positions.append(wanted)
        boxes.append([0, 0, master["width"] / U, master["height"] / U])
        trim = master["trim"]
        trims.append([trim["x"] / U, (master["height"]-trim["y"]-trim["height"]) / U,
                      (trim["x"]+trim["width"]) / U, (master["height"]-trim["y"]) / U])

    for name, counts in phases:
        if mode == "break" and name == "short":
            append(name, [])  # Original simultaneous zero-height break nodes.
        # Width feedback keeps one root-table grid at the narrowest selected
        # width; original Japanese paragraphs therefore split four/four in both
        # scopes. Actual column origins still use each physical master's width.
        words = ["左側右側", "左側右側"] \
            if japanese and mode == "width" else ["Result ", "Result"] if mode == "width" else [text]
        remaining = [words * count for count in counts]
        once = False
        while not once or any(remaining):
            once = True
            _, body = frame(name)
            repeat = int(mode == "header" and name == "short")
            capacity = body["height"] // line - repeat
            assert capacity > 0
            wanted = []
            for col, pending in enumerate(remaining):
                selected = pending[:capacity]
                del pending[:capacity]
                for row, word in enumerate(selected):
                    wanted.append((word, body["x"] + col * body["width"] // len(remaining),
                                   body["y"] + (row + repeat) * line + baseline))
            append(name, wanted)
        if mode == "span" and name == "short":
            # The authored source order places the four-line spanning-cell
            # deficit in the last covered row before the two-line first-row
            # cell is measured. The resulting six-line row bands retain one
            # final geometry-only continuation after all original text paints.
            assert 2*line <= 40*U < 4*line < 80*U
            append(name, [])
    _, body = frame(None)
    append(None, [(text, body["x"], body["y"] + baseline)])
    return mode, japanese, owners, names, positions, boxes, trims, paragraph_owners, breaks


def verify(directory, self_test, require_harano):
    seen = set()
    count = pages = rejected = 0
    for path in sorted(directory.glob("*.json")):
        probe = json.loads(path.read_text())
        mode, japanese, owners, names, wanted, boxes, trims, paragraph_owners, breaks = expectations(probe["source"], probe["text"])
        key = mode, japanese
        assert key not in seen
        seen.add(key)
        assert probe["page_names"] == names, (path.name, probe["page_names"], names)
        data = path.with_suffix(".pdf").read_bytes()
        assert list(hashlib.sha256(data).digest()) == probe["sha256"]
        reader = PdfReader(path.with_suffix(".pdf"), strict=True)
        actual = paragraphs(reader)
        try:
            check_positions(actual, wanted)
        except AssertionError as e:
            raise AssertionError(f"{path.name}: {e}") from e
        assert [list(p["/MediaBox"]) for p in reader.pages] == boxes
        assert [list(p["/TrimBox"]) for p in reader.pages] == trims
        assert not any(p.get("/Annots") for p in reader.pages)
        artifacts = artifact_glyphs(reader)
        for page, glyphs in enumerate(artifacts):
            repeated = [g for g in glyphs if g[3]]
            if mode != "header" or names[page] != "short":
                assert not repeated, (path.name, page)
                continue
            body = selected_master(probe["source"]["page_masters"], page, "short")["body"]
            ascent = round((1151 if japanese else 800) * 12 * U / 1000)
            descent = round((286 if japanese else 200) * 12 * U / 1000)
            line = max(16 * U, ascent+descent)
            baseline = ascent+(line-ascent-descent)//2
            advance = round((1000 if japanese else 600)*12*U/1000)
            expected = [(char, (body["x"]+col*body["width"]//2+i*advance)/U,
                         (body["y"]+baseline)/U) for col in range(2) for i,char in enumerate(probe["text"])]
            assert len(repeated) == len(expected)
            for (char,x,y,_), (word,left,top) in zip(repeated,expected):
                assert char==word and abs(x-left)<=1/U and abs(y-top)<=1/U, (path.name,char,x,y,word,left,top)
        for placement in probe["placements"]:
            assert names[placement["page"]] == owners[placement["owner"]]
        assert {p["owner"] for p in probe["placements"]} == set(paragraph_owners)
        assert probe["consumed"] == list(range(sum(map(len,wanted))+len(breaks)))
        count += 1
        pages += len(reader.pages)
        if self_test:
            altered = [actual[1:], actual + [[]]]
            for field, value in [(0, "lost"), (1, 999), (2, 999)]:
                changed = copy.deepcopy(actual)
                changed[0][0][field] = value
                altered.append(changed)
            for changed in altered:
                try:
                    check_positions(changed, wanted)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError("accepted altered text, source count or physical placement")
    required = {(mode, False) for mode in MODES}
    if require_harano:
        required |= {(mode, True) for mode in MODES}
    assert required <= seen, seen
    return count, pages, rejected


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--require-harano", action="store_true")
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.self_test, args.require_harano)
    print(f"PASS: {count} named table-transition driver PDFs / {pages} pages / {rejected} alterations rejected")
