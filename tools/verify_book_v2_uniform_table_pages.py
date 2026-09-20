#!/usr/bin/env python3
"""Independently check the uniform table-page fixtures against original source."""
import argparse
import copy
import json
from pathlib import Path

from pypdf import PdfReader
from verify_book_v2_definition_table_pdf import check_positions
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_pdf_assembly import selected_master, source_page_names

U = 65536
MODES = {"flat", "nested", "caption", "width"}


def expectations(wire):
    table, following = wire["document"]["blocks"]
    mode, = [c.removeprefix("uniform-") for c in table["classes"] if c.startswith("uniform-")]
    assert mode in MODES and "appendix" in table["classes"]
    assert not wire["document"]["footnotes"]
    text = wire["text_buffers"][0]["utf8"]
    japanese = not text.isascii()
    assert text == ({"nested": "左右", "width": "左側右側左側右側"}.get(mode, "左側右側") if japanese
                    else {"nested": "Res", "width": "Result Result"}.get(mode, "Result"))
    rules = {r["style_id"]: r for r in wire["style_sheet"]["rules"]}
    assert rules["named-table"]["selector"] == "table.appendix"
    assert rules["short-paragraph"]["selector"] == "paragraph.short"
    for rule, name in [("named-table", "appendix"), ("short-paragraph", "short")]:
        assert rules[rule]["declarations"] == [{"name": "page", "important": False,
                                                "value": {"kind": "string", "value": name}}]

    def paragraph(p, classes):
        assert p["kind"] == "paragraph" and p["classes"] == classes
        child, = p["children"]
        span = child["text_span"]
        assert span["start_byte"] == 0 and span["end_byte"] == len(text.encode())

    def check_table(t, nested):
        assert t["kind"] == "table" and "appendix" in t["classes"] and not t["head"]
        assert t["columns"] == [{"kind": "fraction", "weight": 1}] * 2
        row, = t["body"]
        assert len(row["cells"]) == 2
        for cell in row["cells"]:
            assert cell["colspan"] == cell["rowspan"] == 1
            if nested:
                child, = cell["blocks"]
                check_table(child, False)
            else:
                assert len(cell["blocks"]) == 4
                for p in cell["blocks"]:
                    paragraph(p, ["short"])

    check_table(table, mode == "nested")
    if mode == "caption":
        caption, = table["caption"]
        paragraph(caption, ["short"])
    else:
        assert not table.get("caption")
    paragraph(following, [])
    # The unchanged Harano hhea metrics and the controlled font's hhea metrics;
    # original source uses 12 pt type and a minimum 16 pt line height.
    declarations = {d["name"]: d["value"] for d in rules["paragraph-text"]["declarations"]}
    assert declarations["font_size"]["value"] == 12 * U
    assert declarations["line_height"]["value"] == 16 * U
    ascent = round((1151 if japanese else 800) * 12 * U / 1000)
    descent = round((286 if japanese else 200) * 12 * U / 1000)
    line = max(16 * U, ascent + descent)
    baseline = ascent + (line - ascent - descent) // 2
    names = ["short"] * {"flat": 2, "nested": 2, "caption": 3, "width": 4}[mode] + [None]
    positions, boxes = [], []
    for page, name in enumerate(names):
        master = selected_master(wire["page_masters"], page, name)
        body = master["body"]
        x, y, width = body["x"], body["y"], body["width"]
        boxes.append([0, 0, master["width"] / U, master["height"] / U])
        if name is None:
            positions.append([(text, x, y + baseline)])
            continue
        assert body["height"] == 40 * U
        assert 2 * line <= body["height"] < 3 * line
        if mode == "width":
            assert width == 120 * U and x == 18 * U
            words = [text[:4], text[4:]] if japanese else ["Result ", "Result"]
        else:
            words = [text, text]
        count, offset = (1, 1) if mode == "caption" and page == 0 else (1, 0) if mode == "caption" and page == 2 else (2, 0)
        columns = 4 if mode == "nested" else 2
        found = [(text, x, y + baseline)] if offset else []
        for col in range(columns):
            for i in range(count):
                found.append((words[i], x + col * width // columns, y + (i + offset) * line + baseline))
        positions.append(found)
    return mode, japanese, names, positions, boxes


def verify(directory, self_test, require_harano):
    drivers = {}
    for path in directory.glob("*.driver"):
        value = json.loads(path.read_text())
        drivers.setdefault(tuple(value["display"]), []).append(value)
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob("*.json"):
        probe = json.loads(path.read_text())
        blocks = probe.get("wire", {}).get("document", {}).get("blocks", [])
        if not blocks or not any(c.startswith("uniform-") for c in blocks[0].get("classes", [])):
            continue
        mode, japanese, names, wanted, boxes = expectations(probe["wire"])
        key = mode, japanese
        assert key not in seen, key
        seen.add(key)
        assert source_page_names(probe) == names
        driver, = drivers[tuple(probe["display"])]
        assert driver["assembly"]["page_names"] == names
        for assembly in [probe["relations"]["navigation"]["assembly"], driver["assembly"]]:
            reader = PdfReader(assembly["pdf"], strict=True)
            actual = paragraphs(reader)
            check_positions(actual, wanted)
            assert [list(p["/MediaBox"]) for p in reader.pages] == boxes
            assert not any(p.get("/Annots") for p in reader.pages)
            count += 1
            pages += len(reader.pages)
            if self_test:
                alterations = [actual[1:], actual + [[]]]
                for field, value in [(0, "lost"), (1, 999), (2, 999)]:
                    altered = copy.deepcopy(actual)
                    altered[0][0][field] = value
                    alterations.append(altered)
                for altered in alterations:
                    try:
                        check_positions(altered, wanted)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError("accepted altered source text or physical origin")
                altered = copy.deepcopy(probe)
                altered["relations"]["navigation"]["assembly"]["page_names"][0] = "appendix"
                try:
                    source_page_names(altered)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError("accepted enclosing table name over actual content")
                altered = copy.deepcopy(probe["wire"])
                cell = altered["document"]["blocks"][0]["body"][0]["cells"][0]
                if mode == "nested":
                    cell = cell["blocks"][0]["body"][0]["cells"][0]
                cell["blocks"][0]["classes"] = ["appendix"]
                try:
                    expectations(altered)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError("accepted conflicting original cell scope")
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
    print(f"PASS: {count} uniform table-page PDFs / {pages} pages / {rejected} alterations rejected; "
          "original content scopes, caption/nested columns, physical masters and width reflow verified")
