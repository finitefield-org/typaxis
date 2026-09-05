#!/usr/bin/env python3
"""Independent checks for selected-body diagnostic PDFs, not the full-book gate.

Generate inputs using the production_body_assembly_graph Rust test and
TYPAXIS_BODY_PDF_PROBE_DIR. Requires pypdf, Pillow, Poppler and MuPDF. This
probe deliberately cannot issue a publication, manifest or PDF/UA receipt.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess

from PIL import Image, ImageChops
from pypdf import PdfReader, PdfWriter
from pypdf.generic import ContentStream, NameObject, NumberObject, TextStringObject

SPEECH = "one half equals two quarters"
CASES = {
    "body-tt": ["A B"],
    "body-ttc": ["A B"],
    "body-cff": ["A B"],
    "vmb-formula-only": [SPEECH],
    "vmb-body": [f"A {SPEECH}B\n{SPEECH}", "B"],
    "vmb-body-visible-cff": [f"A {SPEECH}B\n{SPEECH}", "B"],
    "vmb-body-spaced-cff": [f"A {SPEECH} B\n{SPEECH}", "B"],
}


class ProbeFailure(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise ProbeFailure(message)


def run(argv):
    result = subprocess.run(argv, capture_output=True, check=True, timeout=120)
    require(b"warning:" not in result.stderr.lower(), f"tool warning: {result.stderr!r}")
    return result.stdout


def expected_roles(name):
    if name.startswith("body-"):
        return [["/Span"]]
    if name == "vmb-formula-only":
        return [["/Formula"]]
    return [["/Span", "/Formula", "/Span", "/Formula"], ["/Span"]]


def verify_structure(pdf, name):
    reader = PdfReader(pdf, strict=True)
    catalog = reader.trailer["/Root"]
    require(catalog["/MarkInfo"]["/Marked"], "missing MarkInfo")
    require(b"<pdfuaid:part>" not in catalog["/Metadata"].get_data(), "premature PDF/UA claim")
    root = catalog["/StructTreeRoot"]
    nums = root["/ParentTree"]["/Nums"]
    roles = expected_roles(name)
    require(len(reader.pages) == len(roles), "page count")
    require(len(nums) == 2 * len(roles), "ParentTree page count")
    observed = []
    for index, (page, expected) in enumerate(zip(reader.pages, roles)):
        require(page["/StructParents"] == index and nums[index * 2] == index, "StructParents")
        parents = nums[index * 2 + 1]
        require(len(parents) == len(expected), "ParentTree MCID count")
        content = ContentStream(page.get_contents(), reader)
        stack, mcids, anchors, draws = [], [], 0, 0
        font, rendering = None, None
        for operands, operator in content.operations:
            if operator == b"BDC":
                tag, properties = operands
                stack.append((tag, properties))
                if "/MCID" in properties:
                    mcid = int(properties["/MCID"])
                    require(mcid == len(mcids), "dense MCID order")
                    require(mcid < len(expected) and tag == expected[mcid], "semantic role order")
                    node = parents[mcid].get_object()
                    require(node["/S"] == tag, "ParentTree role")
                    kids = node["/K"]
                    if not isinstance(kids, list):
                        kids = [kids]
                    require(any(isinstance(k, dict) and k.get("/MCID") == mcid
                                and k.raw_get("/Pg") == page.indirect_reference for k in kids),
                            "structure MCR page/MCID")
                    if tag == "/Formula":
                        require(node["/Alt"] == SPEECH, "Formula Alt")
                    mcids.append(mcid)
                if "/ActualText" in properties:
                    require(tag == "/Span" and properties["/ActualText"] == SPEECH,
                            "per-occurrence ActualText")
                    require(any(t == "/Formula" and "/MCID" in p for t, p in stack),
                            "ActualText owner")
            elif operator == b"EMC":
                require(bool(stack), "unbalanced EMC")
                stack.pop()
            elif operator == b"Tf":
                font = operands[0]
            elif operator == b"Tr":
                rendering = int(operands[0])
            elif operator == b"Tj" and font == "/PBA":
                require(rendering == 3, "semantic anchor must not paint")
                require(any(p.get("/ActualText") == SPEECH for _, p in stack), "anchor ActualText")
                managed = page["/Resources"]["/Font"]["/PBA"]
                require(managed["/Subtype"] == "/Type3", "anchor font type")
                require(managed["/CharProcs"]["/anchor"].get_data() == b"1000 0 0 0 1000 1000 d1\n",
                        "anchor glyph has unexpected painting operators")
                anchors += 1
            elif operator == b"Do":
                require(any(t == "/Formula" for t, _ in stack), "Form placement owner")
                form = page["/Resources"]["/XObject"][operands[0]]
                require(form["/Subtype"] == "/Form", "vector resource type")
                require(all(token not in form.get_data() for token in (b"/MCID", b"/ActualText", b"/Alt")),
                        "shared Form contains occurrence semantics")
                draws += 1
        require(not stack and len(mcids) == len(expected), "marked-content coverage")
        require(anchors == draws == expected.count("/Formula"), "missing/duplicate formula anchor or Do")
        require(("/PBA" in page["/Resources"]["/Font"]) == (anchors > 0), "page anchor font usage")
        observed.append({"mcids": len(mcids), "form_placements": draws, "semantic_anchors": anchors})
    return observed


def verify_extraction(pdf, name, pdftotext, mutool):
    pages = CASES[name]
    # These are tool framing rules, not normalization of authored whitespace.
    expected_poppler = "\f".join(pages) + "\f"
    expected_mupdf = "".join(p.replace("\n", "\n\n") + "\n\n\f\n" for p in pages)
    actual_poppler = run([pdftotext, "-raw", "-enc", "UTF-8", str(pdf), "-"]).decode()
    actual_mupdf = run([mutool, "draw", "-F", "txt", str(pdf)]).decode()
    differences = []
    for tool, actual, expected in (("Poppler", actual_poppler, expected_poppler),
                                   ("MuPDF", actual_mupdf, expected_mupdf)):
        if actual != expected:
            differences.append(f"{tool}: {actual!r} != {expected!r}")
    require(not differences, f"{name}: exact extraction: " + "; ".join(differences)
            + f"; observed Poppler={actual_poppler!r}; observed MuPDF={actual_mupdf!r}")
    return {"poppler_utf8_sha256": hashlib.sha256(actual_poppler.encode()).hexdigest(),
            "mupdf_utf8_sha256": hashlib.sha256(actual_mupdf.encode()).hexdigest()}


def remove_anchor_text(pdf, output):
    """A render-only counterfactual: retain all original body and SVG painting."""
    writer = PdfWriter(clone_from=pdf)
    for page in writer.pages:
        content = ContentStream(page.get_contents(), writer)
        font = None
        kept = []
        for operands, operator in content.operations:
            if operator == b"Tf":
                font = operands[0]
            if operator == b"Tj" and font == "/PBA":
                continue
            kept.append((operands, operator))
        content.operations = kept
        page.replace_contents(content)
    writer.write(output)


def verify_nonpainting(pdf, name, output, mutool):
    without = output / f"{name}-without-anchor.pdf"
    remove_anchor_text(pdf, without)
    for dpi in (72, 144, 288):
        patterns = [output / f"{name}-{dpi}-%d.png", output / f"{name}-without-anchor-{dpi}-%d.png"]
        for source, pattern in zip((pdf, without), patterns):
            run([mutool, "draw", "-r", str(dpi), "-o", str(pattern), str(source)])
        for page in range(1, len(CASES[name]) + 1):
            with Image.open(str(patterns[0]).replace("%d", str(page))) as a, \
                    Image.open(str(patterns[1]).replace("%d", str(page))) as b:
                require(a.size == b.size and a.mode == b.mode, "raster geometry")
                require(ImageChops.difference(a, b).getbbox() is None, "semantic anchor changed visible ink")
                if name.endswith("-cff"):
                    require(a.convert("L").getextrema()[0] < 128, "outlined body fixture is blank")
                if name.startswith("vmb-body-") and name.endswith("-cff") and page == 1:
                    # Source fixture A has x=11 pt, advance=7.2 pt and baseline
                    # 20.149688720703125 pt. The formula begins beyond 21 pt;
                    # its ink cannot make this independent body region pass.
                    scale = dpi / 72
                    box = (math.floor(10 * scale), math.floor(10 * scale),
                           math.ceil(20 * scale), math.ceil(22 * scale))
                    require(a.convert("L").crop(box).getextrema()[0] < 128,
                            "selected body A has no visible ink")
    return {"dpis": [72, 144, 288], "anchor_removed_pixel_difference": 0}


def verify_mutants(pdf, output, pdftotext, mutool):
    rejected = []
    for mutation in ("missing-do", "wrong-mcid", "wrong-actual-text", "painting-anchor", "missing-body-a"):
        writer = PdfWriter(clone_from=pdf)
        page = writer.pages[0]
        content = ContentStream(page.get_contents(), writer)
        changed, operations = False, []
        for operands, operator in content.operations:
            if not changed:
                if mutation == "missing-body-a" and operator == b"Tj":
                    changed = True
                    continue
                if mutation == "missing-do" and operator == b"Do":
                    changed = True
                    continue
                if mutation == "wrong-mcid" and operator == b"BDC" and "/MCID" in operands[1]:
                    operands[1][NameObject("/MCID")] = NumberObject(99)
                    changed = True
                if mutation == "wrong-actual-text" and operator == b"BDC" and "/ActualText" in operands[1]:
                    operands[1][NameObject("/ActualText")] = TextStringObject("missing formula")
                    changed = True
                if mutation == "painting-anchor" and operator == b"Tr" and operands[0] == 3:
                    operands[0] = NumberObject(0)
                    changed = True
            operations.append((operands, operator))
        require(changed, f"mutant did not modify input: {mutation}")
        content.operations = operations
        page.replace_contents(content)
        path = output / f"mutant-{mutation}.pdf"
        writer.write(path)
        try:
            if mutation == "missing-body-a":
                render_root = output / "missing-body-a-render"
                render_root.mkdir(exist_ok=True)
                verify_nonpainting(path, "vmb-body-visible-cff", render_root, mutool)
            else:
                verify_structure(path, "vmb-body-visible-cff")
        except ProbeFailure:
            rejected.append(mutation)
        else:
            raise ProbeFailure(f"verifier accepted {mutation}")
        if mutation == "wrong-actual-text":
            try:
                verify_extraction(path, "vmb-body-visible-cff", pdftotext, mutool)
            except ProbeFailure:
                rejected.append("wrong-actual-text-extraction")
            else:
                raise ProbeFailure("extractor verifier accepted wrong ActualText")
    return rejected


def tool_identity(executable, version_flag):
    result = subprocess.run([executable, version_flag], capture_output=True, check=True, timeout=30)
    return {"executable": executable, "version": (result.stdout + result.stderr).decode().strip()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe-root", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--pdftotext", default="pdftotext")
    parser.add_argument("--mutool", default="mutool")
    args = parser.parse_args()
    args.output_root.mkdir(parents=True, exist_ok=True)
    results, failures = {}, []
    for name in CASES:
        pdf = args.probe_root / f"{name}.pdf"
        package = args.probe_root / f"{name}.package.json"
        result = {"pdf_sha256": hashlib.sha256(pdf.read_bytes()).hexdigest(),
                  "package_sha256": hashlib.sha256(package.read_bytes()).hexdigest()}
        checks = {
            "structure": lambda: verify_structure(pdf, name),
            "extraction": lambda: verify_extraction(pdf, name, args.pdftotext, args.mutool),
            "render": lambda: verify_nonpainting(pdf, name, args.output_root, args.mutool),
        }
        for phase, check in checks.items():
            try:
                result[phase] = {"passed": True, "observed": check()}
            except ProbeFailure as error:
                result[phase] = {"passed": False, "error": str(error)}
                failures.append({"case": name, "phase": phase, "error": str(error)})
        results[name] = result
    mutants = verify_mutants(args.probe_root / "vmb-body-visible-cff.pdf", args.output_root,
                             args.pdftotext, args.mutool)
    observed = {"scope": "selected-body-diagnostic-probe", "public_build_verified": False,
                "full_book_verified": False, "cases": results, "failures": failures,
                "negative_probes_rejected": mutants,
                "tools": {"poppler": tool_identity(args.pdftotext, "-v"),
                          "mupdf": tool_identity(args.mutool, "-v")}}
    (args.output_root / "observed.json").write_text(json.dumps(observed, indent=2) + "\n")
    print(json.dumps({"cases": len(results), "failed_checks": len(failures),
                      "negative_probes_rejected": len(mutants), "scope": observed["scope"]}))
    return 1 if failures else 0



if __name__ == "__main__":
    raise SystemExit(main())
