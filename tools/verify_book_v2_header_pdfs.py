#!/usr/bin/env python3
"""Compare embedded variant PDF fonts and actual CID paints to resource probes.

Run after verify_book_v2_pdf_assembly.py and verify_book_v2_header_resources.py.
This links independently verified subset programs to real PDF page content.
"""
import argparse
from collections import Counter
import hashlib
import json
import io
from pathlib import Path
import re

from pypdf import PdfReader, PdfWriter
from pypdf.generic import ByteStringObject, ContentStream, DecodedStreamObject, NameObject
from verify_book_v2_font_objects import widths
from verify_book_v2_pdf_assembly import operations, original


def verify(path, facts, reader=None):
    reader = PdfReader(path, strict=True) if reader is None else reader
    expected_hashes = [f["subset_sha256"] for f in facts["fonts"]]
    assert len(expected_hashes) == len(set(expected_hashes))
    uses = Counter((u["font"], cid) for u in facts["uses"] for cid in u["cids"])
    painted, artifacts = Counter(), Counter()
    for page in reader.pages:
        fonts = {}
        for name, ref in page["/Resources"]["/Font"].items():
            font = ref.get_object()
            if name == "/BMA":
                assert font["/Subtype"] == "/Type3"
                continue
            descendant = font["/DescendantFonts"][0].get_object()
            descriptor = descendant["/FontDescriptor"]
            cff = "/FontFile3" in descriptor
            program = descriptor["/FontFile3" if cff else "/FontFile2"].get_data()
            digest = hashlib.sha256(program).hexdigest()
            assert digest in expected_hashes
            index = expected_hashes.index(digest)
            fonts[name] = index
            info = facts["fonts"][index]
            actual_widths = widths(descendant["/W"])
            for b in info["bindings"]:
                assert actual_widths.get(b["cid"], descendant.get("/DW", 1000)) == b["width"]
            if not cff:
                mapping = descendant["/CIDToGIDMap"].get_data()
                for b in info["bindings"]:
                    offset = b["cid"] * 2
                    assert int.from_bytes(mapping[offset:offset + 2], "big") == b["subset"]
            cmap = font["/ToUnicode"].get_data()
            actual = {}
            for block in re.findall(rb"beginbfchar\s+(.*?)\s+endbfchar", cmap, re.S):
                for cid, text in re.findall(rb"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>", block):
                    actual[int(cid, 16)] = bytes.fromhex(text.decode()).decode("utf-16-be")
            assert actual == {b["cid"]: b["unicode"] for b in info["bindings"] if b["unicode"] is not None}
        assert set(fonts.values()) == set(range(len(expected_hashes)))
        scopes, current = [], None
        for args, op in operations(page.get_contents().get_data(), reader):
            if op in (b"BMC", b"BDC"):
                scopes.append(args[0] == "/Artifact")
            elif op == b"EMC":
                scopes.pop()
            elif op == b"Tf":
                current = args[0]
            elif op in (b"Tj", b"TJ", b"'", b'"'):
                if current == "/BMA":
                    continue
                assert current in fonts
                values = args[0] if op == b"TJ" else [args[-1]]
                for value in values:
                    if isinstance(value, (int, float)):
                        continue
                    raw = original(value)
                    assert len(raw) % 2 == 0
                    for start in range(0, len(raw), 2):
                        key = (fonts[current], int.from_bytes(raw[start:start + 2], "big"))
                        painted[key] += 1
                        if any(scopes):
                            artifacts[key] += 1
        assert not scopes
    assert painted == uses, (path.name, "actual CID paints differ from selected uses")
    if facts["repeated_only_gid"] is not None:
        gid = facts["repeated_only_gid"]
        keys = [(i, b["cid"]) for i, f in enumerate(facts["fonts"]) for b in f["bindings"] if b["gid"] == gid]
        assert keys and all(painted[k] > 0 and painted[k] == artifacts[k] for k in keys)
    return len(reader.pages), sum(painted.values())


def mutated(path, mode):
    # Mutate the writer's cloned page tree. Reader.pages can be flattened copies
    # whose direct Contents replacement is lost by clone_document_from_reader.
    reader = PdfWriter()
    reader.clone_document_from_reader(PdfReader(path, strict=True))
    changed = False
    if mode == "program":
        for ref in reader.pages[0]["/Resources"]["/Font"].values():
            font = ref.get_object()
            if "/DescendantFonts" in font:
                descriptor = font["/DescendantFonts"][0].get_object()["/FontDescriptor"]
                key = "/FontFile3" if "/FontFile3" in descriptor else "/FontFile2"
                stream = DecodedStreamObject()
                stream.set_data(b"invalid font program")
                descriptor[NameObject(key)] = stream
                changed = True
                break
    else:
        for page in reader.pages:
            stream = ContentStream(page.get_contents(), reader)
            current = None
            ops = stream.operations
            for args, op in ops:
                if op == b"Tf":
                    current = args[0]
                if mode == "cid" and not changed and current != "/BMA" and op == b"Tj":
                    raw = original(args[0])
                    if raw:
                        args[0] = ByteStringObject(b"\0\0" + raw[2:])
                        changed = True
                if mode == "artifact" and op in (b"BMC", b"BDC") and args[0] == "/Artifact":
                    args[0] = NameObject("/Span")
                    changed = True
            stream.operations = ops
            page[NameObject("/Contents")] = stream
    assert changed
    data = io.BytesIO()
    reader.write(data)
    return PdfReader(io.BytesIO(data.getvalue()), strict=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdf_probes", type=Path)
    parser.add_argument("resource_probes", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    resources = {p.stem: json.loads(p.read_text()) for p in args.resource_probes.glob("*.json")}
    assert resources
    seen, pages, glyphs, rejected = set(), 0, 0, 0
    for path in args.pdf_probes.glob("*.json"):
        proof = json.loads(path.read_text())
        if "display" not in proof or "relations" not in proof:
            continue
        key = bytes(proof["display"]).hex()
        if key not in resources:
            continue
        assert key not in seen
        pdf = Path(proof["relations"]["navigation"]["assembly"]["pdf"])
        pc, gc = verify(pdf, resources[key])
        if args.self_test:
            modes = ["program", "cid"] + (["artifact"] if resources[key]["repeated_only_gid"] is not None else [])
            for mode in modes:
                try:
                    verify(pdf, resources[key], mutated(pdf, mode))
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError((pdf.name, mode, "modified PDF accepted"))
        seen.add(key)
        pages += pc
        glyphs += gc
    assert seen == resources.keys(), "missing variant PDFs"
    print(f"PASS: {len(seen)} variant PDFs / {pages} pages / {glyphs} actual CID glyph paints; "
          f"{rejected} modified-PDF rejections; embedded programs, widths, CID maps and ToUnicode match verified resources")


if __name__ == "__main__":
    main()
