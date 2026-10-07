#!/usr/bin/env python3
"""Check closed named-footnote driver fixtures against their original input.

The oracle uses source scopes, authored masters and pinned font metrics. Captured
placement and continuation records are never used as expected geometry.
"""
import argparse
import copy
import hashlib
import json
import math
import re
from collections import Counter
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ContentStream
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_pdf_assembly import selected_master, source_page_owner_names

U = 65536
MODES = {"aligned", "serial", "return", "scope", "forced", "flat", "nested", "header",
         "span", "caption", "empty-child", "width", "two-notes"}


def header_text(reader):
    result = []
    for page in reader.pages:
        stack, chars = [], []
        cmap = None
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op in (b'BDC', b'BMC'):
                stack.append(op == b'BDC' and args[0] == '/Artifact'
                             and args[1].get('/Subtype') == '/Header')
            elif op == b'EMC':
                stack.pop()
            elif op == b'Tf':
                font = page['/Resources']['/Font'][args[0]].get_object()
                pairs = re.findall(rb'<([0-9A-F]+)> <([0-9A-F]+)>',
                    font['/ToUnicode'].get_data().split(b'beginbfchar', 1)[1])
                cmap = {int(a, 16): bytes.fromhex(b.decode()).decode('utf-16-be') for a, b in pairs}
            elif op == b'Tj' and any(stack):
                raw = args[0].original_bytes if hasattr(args[0], 'original_bytes') else bytes(args[0])
                assert len(raw) == 2
                chars.append(cmap[int.from_bytes(raw, 'big')])
        assert not stack
        result.append(''.join(chars))
    return result


def source_facts(wire, text):
    body, = wire["document"]["blocks"]
    mode, = [c.removeprefix("named-note-") for c in body["classes"]
             if c.startswith("named-note-")]
    assert mode in MODES
    japanese = not text.isascii()
    assert text == ("左右" if japanese else "Res" if mode == "nested" else "Result")
    notes = wire["document"]["footnotes"]
    assert len(notes) == (2 if mode == "two-notes" else 1)
    rules = {r["style_id"]: r for r in wire["style_sheet"]["rules"]}
    style = {d["name"]: d["value"] for d in rules["paragraph-text"]["declarations"]}
    assert style["font_size"]["value"] == 12 * U
    assert style["line_height"]["value"] == 16 * U
    ascent = round((1151 if japanese else 800) * 12 * U / 1000)
    descent = round((286 if japanese else 200) * 12 * U / 1000)
    line = max(16 * U, ascent + descent)
    baseline = ascent + (line - ascent - descent) // 2
    capacity_a, capacity_b = 64 * U // line, 32 * U // line
    names = ["appendix"]
    if mode == "aligned":
        names *= math.ceil(7 / capacity_a)
    elif mode == "return":
        names += ["short"] * math.ceil(2 / capacity_b) + ["appendix"]
    elif mode in {"flat", "nested"}:
        names += ["short"] * math.ceil(2 / capacity_b)
    elif mode == "header":
        names += ["short"] * math.ceil(2 / (48 * U // line - 1))
    elif mode == "span":
        # The source rowspan leaves four line heights of row bands after the
        # first two-line phase. The last continuation has no original paint.
        names += ["short"] * math.ceil(4 * line / (32 * U))
    elif mode == "empty-child":
        names = ["short"] * math.ceil(4 / capacity_b) + ["appendix"]
    elif mode == "two-notes":
        # Reserve a real first line for each newly referenced note. Started
        # carries retain source order and may wait when only one line fits.
        names = (["appendix"] * 2 + ["short"] * 5 if japanese else
                 ["appendix"] + ["short"] * 2 + ["appendix", "short"])
    else:
        names += ["short"] * math.ceil(4 / capacity_b)
    owners = source_page_owner_names(wire, True)
    expected = Counter()
    note_paragraphs = []

    def visit(value):
        if isinstance(value, list):
            for child in value:
                visit(child)
        elif isinstance(value, dict):
            if value.get("kind") == "paragraph":
                child, = value["children"]
                assert child["kind"] == "text"
                assert child["text_span"]["start_byte"] == 0
                assert child["text_span"]["end_byte"] == len(text.encode())
                name = owners[value["node_id"]]
                assert name in {"appendix", "short"}
                expected[name, text] += 1
                note_paragraphs.append(value)
            for field in ("blocks", "caption", "head", "body", "cells"):
                if field in value:
                    visit(value[field])

    visit(notes)
    assert len(note_paragraphs) == {"flat": 8, "nested": 16, "header": 6,
                                   "span": 8, "caption": 9, "empty-child": 8,
                                   "two-notes": 11}.get(mode, 7)
    body_name = source_page_owner_names(wire)[body["node_id"]]
    assert body_name == names[0]
    expected[body_name, text] += 1
    return mode, japanese, names, expected, line, baseline, len(notes)


def check(actual, wire, text, facts):
    mode, japanese, names, expected, line, baseline, note_count = facts
    assert len(actual) == len(names), (len(actual), names)
    found = Counter()
    labels = Counter()
    for index, (name, groups) in enumerate(zip(names, actual)):
        master = selected_master(wire["page_masters"], index, name)
        body, notes = master["body"], master["footnote"]
        for word, x, y in groups:
            assert word == text or word in {str(i + 1) for i in range(note_count)}
            in_body = body["y"] / U <= y <= (body["y"] + body["height"]) / U
            in_notes = notes["y"] / U <= y <= (notes["y"] + notes["height"]) / U
            assert in_body != in_notes, (index, word, x, y)
            region = body if in_body else notes
            assert region["x"] / U <= x < (region["x"] + region["width"]) / U
            if in_body:
                assert index == 0
                assert abs(y - (body["y"] + baseline) / U) <= 1 / U
                if word == text:
                    assert abs(x - body["x"] / U) <= 1 / U
            if word == text:
                found[name, word] += 1
            else:
                labels[word, in_body] += 1
        # Original paint belongs to the physical note frame. No geometry-only
        # continuation gains text merely to make the page nonempty.
        if mode in {"span", "empty-child"} and index == len(names) - 1:
            assert not groups
    assert found == expected, (found, expected)
    assert labels == Counter({(str(i + 1), body): 1
                              for i in range(note_count) for body in (True, False)})


def verify(directory, self_test, require_harano):
    seen = set()
    count = pages = rejected = 0
    for path in sorted(directory.glob("*.json")):
        probe = json.loads(path.read_text())
        wire, text = probe["source"], probe["text"]
        facts = source_facts(wire, text)
        mode, japanese, names, _, _, _, _ = facts
        key = mode, japanese
        assert key not in seen
        seen.add(key)
        assert probe["page_names"] == names, (path.name, probe["page_names"], names)
        data = path.with_suffix(".pdf").read_bytes()
        assert list(hashlib.sha256(data).digest()) == probe["sha256"]
        reader = PdfReader(path.with_suffix(".pdf"), strict=True)
        actual = paragraphs(reader)
        check(actual, wire, text, facts)
        for index, (page, name) in enumerate(zip(reader.pages, names)):
            master = selected_master(wire["page_masters"], index, name)
            assert list(page["/MediaBox"]) == [0, 0, master["width"] / U, master["height"] / U]
            trim = master["trim"]
            assert list(page["/TrimBox"]) == [trim["x"] / U,
                (master["height"] - trim["y"] - trim["height"]) / U,
                (trim["x"] + trim["width"]) / U, (master["height"] - trim["y"]) / U]
        for index, repeated in enumerate(header_text(reader)):
            if mode == "header" and names[index] == "short":
                assert repeated == text * 2
            else:
                assert not repeated
        count += 1
        pages += len(reader.pages)
        if self_test:
            nonempty = next(index for index, groups in enumerate(actual) if groups)
            changes = [actual[1:], actual + [[]]]
            for field, value in [(0, "lost"), (1, 999), (2, 999)]:
                changed = copy.deepcopy(actual)
                changed[nonempty][0][field] = value
                changes.append(changed)
            changed = copy.deepcopy(actual)
            changed[nonempty].append(changed[nonempty][0])
            changes.append(changed)
            for changed in changes:
                try:
                    check(changed, wire, text, facts)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError("accepted altered original text, note marker or physical placement")
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
    print(f"PASS: {count} named footnote driver PDFs / {pages} pages / {rejected} alterations rejected")
