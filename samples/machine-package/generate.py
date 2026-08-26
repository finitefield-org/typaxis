#!/usr/bin/env python3
"""Regenerate the deterministic MI1-16 machine package fixture bundle."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import shutil
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
PROFILE = "typaxis.machine-pdf/paragraph-1"
EMPTY_SHA256 = hashlib.sha256(b"").hexdigest()
PHRASE = "Typaxis machine input"
MATRIX_TEST = "machine_tests::matrix_{row:02d}_{name}"


def jcs(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


def sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def checksum(payload: bytes) -> int:
    padded = payload + b"\0" * ((-len(payload)) % 4)
    return sum(
        int.from_bytes(padded[offset : offset + 4], "big")
        for offset in range(0, len(padded), 4)
    ) & 0xFFFF_FFFF


def synthetic_ascii_ttf() -> bytes:
    glyphs = 96
    head = bytearray(54)
    head[0:4] = (0x0001_0000).to_bytes(4, "big")
    head[12:16] = (0x5F0F_3CF5).to_bytes(4, "big")
    head[18:20] = (1000).to_bytes(2, "big")
    head[38:40] = (-200).to_bytes(2, "big", signed=True)
    head[40:42] = (1000).to_bytes(2, "big", signed=True)
    head[42:44] = (800).to_bytes(2, "big", signed=True)
    head[46:48] = (8).to_bytes(2, "big")
    head[48:50] = (2).to_bytes(2, "big", signed=True)
    head[50:52] = (1).to_bytes(2, "big", signed=True)

    hhea = bytearray(36)
    hhea[0:4] = (0x0001_0000).to_bytes(4, "big")
    hhea[4:6] = (800).to_bytes(2, "big", signed=True)
    hhea[6:8] = (-200).to_bytes(2, "big", signed=True)
    hhea[10:12] = (600).to_bytes(2, "big")
    hhea[34:36] = glyphs.to_bytes(2, "big")

    maxp = bytearray(32)
    maxp[0:4] = (0x0001_0000).to_bytes(4, "big")
    maxp[4:6] = glyphs.to_bytes(2, "big")

    hmtx = bytearray()
    for glyph in range(glyphs):
        hmtx.extend((300 if glyph == 1 else 600).to_bytes(2, "big"))
        hmtx.extend((0).to_bytes(2, "big", signed=True))

    cmap = bytearray(44)
    cmap[2:4] = (1).to_bytes(2, "big")
    cmap[4:6] = (3).to_bytes(2, "big")
    cmap[6:8] = (1).to_bytes(2, "big")
    cmap[8:12] = (12).to_bytes(4, "big")
    cmap[12:14] = (4).to_bytes(2, "big")
    cmap[14:16] = (32).to_bytes(2, "big")
    cmap[18:20] = (4).to_bytes(2, "big")
    cmap[20:22] = (4).to_bytes(2, "big")
    cmap[22:24] = (1).to_bytes(2, "big")
    cmap[26:28] = (0x007E).to_bytes(2, "big")
    cmap[28:30] = (0xFFFF).to_bytes(2, "big")
    cmap[32:34] = (0x0020).to_bytes(2, "big")
    cmap[34:36] = (0xFFFF).to_bytes(2, "big")
    cmap[36:38] = (-31).to_bytes(2, "big", signed=True)
    cmap[38:40] = (1).to_bytes(2, "big", signed=True)

    postscript_name = b"TypaxisSynthetic"
    name = bytearray(18 + len(postscript_name) * 2)
    name[2:4] = (1).to_bytes(2, "big")
    name[4:6] = (18).to_bytes(2, "big")
    name[6:8] = (3).to_bytes(2, "big")
    name[8:10] = (1).to_bytes(2, "big")
    name[10:12] = (0x0409).to_bytes(2, "big")
    name[12:14] = (6).to_bytes(2, "big")
    name[14:16] = (len(postscript_name) * 2).to_bytes(2, "big")
    for index, byte in enumerate(postscript_name):
        name[19 + index * 2] = byte

    post = bytearray(32)
    post[0:4] = (0x0003_0000).to_bytes(4, "big")
    tables = {
        b"cmap": bytes(cmap),
        b"glyf": b"",
        b"head": bytes(head),
        b"hhea": bytes(hhea),
        b"hmtx": bytes(hmtx),
        b"loca": bytes((glyphs + 1) * 4),
        b"maxp": bytes(maxp),
        b"name": bytes(name),
        b"post": bytes(post),
    }
    count = len(tables)
    directory_length = 12 + count * 16
    payload_length = sum((len(value) + 3) & ~3 for value in tables.values())
    output = bytearray(directory_length + payload_length)
    output[0:4] = (0x0001_0000).to_bytes(4, "big")
    output[4:6] = count.to_bytes(2, "big")
    selector = count.bit_length() - 1
    search = 16 * (1 << selector)
    output[6:8] = search.to_bytes(2, "big")
    output[8:10] = selector.to_bytes(2, "big")
    output[10:12] = (count * 16 - search).to_bytes(2, "big")
    payload_offset = directory_length
    head_adjustment = None
    for index, (tag, payload) in enumerate(sorted(tables.items())):
        record = 12 + index * 16
        output[record : record + 4] = tag
        output[record + 4 : record + 8] = checksum(payload).to_bytes(4, "big")
        output[record + 8 : record + 12] = payload_offset.to_bytes(4, "big")
        output[record + 12 : record + 16] = len(payload).to_bytes(4, "big")
        output[payload_offset : payload_offset + len(payload)] = payload
        if tag == b"head":
            head_adjustment = payload_offset + 8
        payload_offset = (payload_offset + len(payload) + 3) & ~3
    assert head_adjustment is not None
    adjustment = (0xB1B0_AFBA - checksum(bytes(output))) & 0xFFFF_FFFF
    output[head_adjustment : head_adjustment + 4] = adjustment.to_bytes(4, "big")
    return bytes(output)


def single_face_ttc(ttf: bytes) -> bytes:
    sfnt = bytearray(ttf)
    table_count = int.from_bytes(sfnt[4:6], "big")
    header_length = 16
    for index in range(table_count):
        field = 12 + index * 16 + 8
        offset = int.from_bytes(sfnt[field : field + 4], "big")
        sfnt[field : field + 4] = (offset + header_length).to_bytes(4, "big")
    return (
        b"ttcf"
        + (0x0001_0000).to_bytes(4, "big")
        + (1).to_bytes(4, "big")
        + header_length.to_bytes(4, "big")
        + bytes(sfnt)
    )


def source_record(source: bytes, uri: str = "sources/blank.json", source_id: int = 0) -> dict[str, Any]:
    return {
        "source_id": source_id,
        "uri": uri,
        "utf8_byte_length": len(source),
        "sha256": sha256(source),
    }


def span(start: int = 0, end: int = 0) -> dict[str, int]:
    return {"source_id": 0, "start_byte": start, "end_byte": end}


def base_package(
    *,
    contract: str = "typaxis.contract/1.1",
    source: bytes = b"",
    master_id: str = "default",
) -> dict[str, Any]:
    return {
        "contract": contract,
        "coordinate_unit": "pdf_point_1_65536",
        "sources": [source_record(source)],
        "text_buffers": [],
        "document": {"node_id": 0, "blocks": [], "footnotes": []},
        "style_sheet": {"rules": []},
        "page_masters": {
            "default_master_id": master_id,
            "masters": [
                {
                    "master_id": master_id,
                    "width": 10_000_000,
                    "height": 10_000_000,
                    "body": {
                        "x": 655_360,
                        "y": 655_360,
                        "width": 8_689_280,
                        "height": 8_689_280,
                    },
                    "header": None,
                    "footer": None,
                    "footnote": None,
                }
            ],
            "selection_rules": [],
        },
        "resources": {"font_faces": [], "images": []},
    }


def style_rule(style_id: str, selector: str, source_order: int, family: str) -> dict[str, Any]:
    return {
        "style_id": style_id,
        "extends": None,
        "selector": selector,
        "source_order": source_order,
        "declarations": [
            {
                "name": "font_family",
                "value": {"kind": "font_family_list", "families": [family]},
                "important": False,
            },
            {
                "name": "font_size",
                "value": {"kind": "length", "value": 786_432},
                "important": False,
            },
            {
                "name": "line_height",
                "value": {"kind": "length", "value": 917_504},
                "important": False,
            },
            {
                "name": "page",
                "value": {"kind": "keyword", "value": "auto"},
                "important": False,
            },
        ],
    }


def combined_package(ttf: bytes, ttc: bytes) -> dict[str, Any]:
    source = PHRASE.encode("utf-8")
    package = base_package(source=source, master_id="combined")
    package["page_masters"]["masters"][0]["width"] = 30_000_000
    package["page_masters"]["masters"][0]["body"]["width"] = 28_689_280
    package["text_buffers"] = [
        {
            "text_id": 0,
            "utf8": PHRASE,
            "mappings": [
                {
                    "text_range": {"start_byte": 0, "end_byte": len(source)},
                    "kind": "identity",
                    "source_span": span(0, len(source)),
                }
            ],
        }
    ]
    package["document"]["blocks"] = [
        {
            "kind": "heading",
            "node_id": 1,
            "span": span(0, len(source)),
            "classes": [],
            "level": 1,
            "anchor_id": None,
            "children": [
                {
                    "kind": "text",
                    "node_id": 2,
                    "span": span(0, len(source)),
                    "text_span": {
                        "text_id": 0,
                        "start_byte": 0,
                        "end_byte": len(source),
                    },
                },
                {"kind": "soft_break", "node_id": 3, "span": span(len(source), len(source))},
                {"kind": "hard_break", "node_id": 4, "span": span(len(source), len(source))},
            ],
        },
        {
            "kind": "paragraph",
            "node_id": 5,
            "span": span(),
            "classes": [],
            "children": [
                {"kind": "anchor", "node_id": 6, "span": span(), "anchor_id": "page-target"},
                {
                    "kind": "reference",
                    "node_id": 7,
                    "span": span(),
                    "target": "page-target",
                    "format": "page",
                }
            ],
        },
    ]
    package["style_sheet"]["rules"] = [
        style_rule("heading-style", "heading", 0, "Collection"),
        style_rule("paragraph-style", "paragraph", 1, "Standalone"),
    ]
    package["resources"]["font_faces"] = [
        {
            "font_face_id": 0,
            "family": "Standalone",
            "uri": "fonts/body.ttf",
            "face_index": 0,
            "expected_sha256": sha256(ttf),
        },
        {
            "font_face_id": 1,
            "family": "Collection",
            "uri": "fonts/collection.ttc",
            "face_index": 0,
            "expected_sha256": sha256(ttc),
        },
    ]
    return package


ADVERTISED_COVERAGE = sorted(
    [
        "block:heading",
        "block:paragraph",
        "font_format:sfnt-truetype-glyf",
        "font_format:ttc-truetype-glyf",
        "inline:anchor",
        "inline:hard_break",
        "inline:reference",
        "inline:soft_break",
        "inline:text",
        "page_master:default",
        "page_value:auto",
        "pdf_feature:named-destinations",
        "pdf_feature:text-extraction",
        "reference_format:page",
        "source_closure:entry_only",
        "style_block_type:heading",
        "style_block_type:paragraph",
        "style_property:font_family",
        "style_property:font_size",
        "style_property:line_height",
        "style_property:page",
        "style_selector:heading",
        "style_selector:paragraph",
    ]
)


def side_effects(
    *, package: bool, source: bool, resource: bool, layout: bool, pdf: bool
) -> dict[str, bool]:
    return {
        "package_read": package,
        "source_read": source,
        "resource_opened": resource,
        "layout_started": layout,
        "pdf_started": pdf,
    }


def expectation(
    fixture_id: str,
    *,
    contract: str = "typaxis.contract/1.1",
    fixture_class: str,
    exit_code: int,
    primary_code: str | None,
    location: str | None,
    package_progress: str,
    sources_progress: str,
    resources_progress: str,
    effects: dict[str, bool],
    visible: list[str],
    page_count: int | None = None,
    text: str | None = None,
    coverage: list[str] | None = None,
    resource_hashes: list[dict[str, Any]] | None = None,
    arguments: list[str] | None = None,
) -> dict[str, Any]:
    if arguments is None:
        arguments = [
            "job/document-package.json",
            "-o",
            "$OUTPUT/output.pdf",
            "--package-root",
            "job",
            "--profile",
            PROFILE,
            "--resource-root",
            "job",
            "--emit-build-manifest",
            "$OUTPUT/manifest.json",
            "--emit-diagnostics",
            "$OUTPUT/diagnostics.json",
        ]
    return {
        "advertised_item_coverage": coverage or [],
        "arguments": arguments,
        "command": "build-package",
        "contract": contract,
        "expected": {
            "exit_code": exit_code,
            "location": location,
            "manifest_progress": {
                "package": package_progress,
                "resources": resources_progress,
                "sources": sources_progress,
            },
            "normalized_extracted_text": text,
            "page_count": page_count,
            "primary_code": primary_code,
            "side_effects": effects,
            "visible_artifacts": sorted(visible),
        },
        "fixture_class": fixture_class,
        "fixture_id": fixture_id,
        "package": "job/document-package.json",
        "profile": PROFILE,
        "resource_hashes": resource_hashes or [],
    }


def write_fixture(
    relative: str,
    package: bytes,
    expected: dict[str, Any],
    *,
    sources: dict[str, bytes] | None = None,
    resources: dict[str, bytes] | None = None,
) -> str:
    directory = ROOT / relative
    (directory / "job").mkdir(parents=True, exist_ok=True)
    (directory / "job" / "document-package.json").write_bytes(package)
    for uri, payload in (sources or {"sources/blank.json": b""}).items():
        target = directory / "job" / uri
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payload)
    for uri, payload in (resources or {}).items():
        target = directory / "job" / uri
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payload)
    (directory / "expected.json").write_bytes(jcs(expected))
    return f"{relative}/expected.json"


def valid_outcome(fixture_id: str, *, contract: str, text: str | None = "") -> dict[str, Any]:
    return expectation(
        fixture_id,
        contract=contract,
        fixture_class="positive",
        exit_code=0,
        primary_code=None,
        location=None,
        package_progress="validated",
        sources_progress="admitted",
        resources_progress="admitted",
        effects=side_effects(package=True, source=True, resource=False, layout=True, pdf=True),
        visible=["diagnostics", "manifest", "pdf"],
        page_count=1,
        text=text,
        coverage=["page_master:default", "source_closure:entry_only"],
    )


def invalid_outcome(
    fixture_id: str,
    code: str,
    *,
    exit_code: int = 1,
    location: str = "global",
    package_progress: str = "raw",
    sources_progress: str = "none",
    resources_progress: str = "none",
    package_read: bool = True,
    source_read: bool = False,
    fixture_class: str = "negative",
    arguments: list[str] | None = None,
) -> dict[str, Any]:
    return expectation(
        fixture_id,
        fixture_class=fixture_class,
        exit_code=exit_code,
        primary_code=code,
        location=location,
        package_progress=package_progress,
        sources_progress=sources_progress,
        resources_progress=resources_progress,
        effects=side_effects(
            package=package_read,
            source=source_read,
            resource=False,
            layout=False,
            pdf=False,
        ),
        visible=["diagnostics", "manifest"] if exit_code != 2 else [],
        arguments=arguments,
    )


def maximum_json_depth(value: Any) -> int:
    maximum = 0
    stack = [(value, 0)]
    while stack:
        current, depth = stack.pop()
        if isinstance(current, dict):
            depth += 1
            maximum = max(maximum, depth)
            stack.extend((child, depth) for child in current.values())
        elif isinstance(current, list):
            depth += 1
            maximum = max(maximum, depth)
            stack.extend((child, depth) for child in current)
    return maximum


def rejected_content_package() -> dict[str, Any]:
    package = base_package(master_id="unsupported-content")
    package["document"]["blocks"] = [
        {
            "kind": "paragraph",
            "node_id": 1,
            "span": span(),
            "classes": [],
            "children": [
                {"kind": "anchor", "node_id": 2, "span": span(), "anchor_id": "target"},
                {
                    "kind": "emphasis",
                    "node_id": 3,
                    "span": span(),
                    "children": [],
                },
                {
                    "kind": "link",
                    "node_id": 4,
                    "span": span(),
                    "target": {"kind": "uri", "uri": "https://example.com"},
                    "children": [],
                },
                {
                    "kind": "reference",
                    "node_id": 5,
                    "span": span(),
                    "target": "target",
                    "format": "text",
                },
                {
                    "kind": "footnote_reference",
                    "node_id": 6,
                    "span": span(),
                    "footnote_id": "note",
                },
            ],
        },
        {"kind": "page_break", "node_id": 7, "span": span(), "classes": []},
    ]
    package["document"]["footnotes"] = [
        {"footnote_id": "note", "node_id": 8, "span": span(), "blocks": []}
    ]
    return package


def rejected_style_package() -> dict[str, Any]:
    package = base_package(master_id="a")
    package["style_sheet"]["rules"] = [
        {
            "style_id": "list-style",
            "extends": None,
            "selector": "list",
            "source_order": 0,
            "declarations": [
                {
                    "name": "page",
                    "value": {"kind": "string", "value": "special"},
                    "important": False,
                }
            ],
        },
        {
            "style_id": "class-style",
            "extends": None,
            "selector": "paragraph.fixture",
            "source_order": 1,
            "declarations": [],
        },
    ]
    master_a = package["page_masters"]["masters"][0]
    master_a["header"] = {"x": 0, "y": 0, "width": 100, "height": 10}
    package["page_masters"]["masters"].append(
        copy.deepcopy(base_package(master_id="b")["page_masters"]["masters"][0])
    )
    package["page_masters"]["selection_rules"] = [
        {
            "master_id": "a",
            "parity": "any",
            "first": None,
            "named_page": None,
            "source_order": 0,
        }
    ]
    return package


def rejected_image_package() -> dict[str, Any]:
    package = base_package(master_id="unsupported-image")
    package["resources"]["images"] = [
        {"image_id": 0, "uri": "images/missing.png", "expected_sha256": None}
    ]
    return package


def reset_generated_tree() -> None:
    for name in ["capabilities.json", "profiles", "invalid", "scenarios", "matrices"]:
        target = ROOT / name
        if target.is_dir() and not target.is_symlink():
            shutil.rmtree(target)
        elif target.exists() or target.is_symlink():
            target.unlink()


def main() -> None:
    reset_generated_tree()
    ttf = synthetic_ascii_ttf()
    ttc = single_face_ttc(ttf)
    capabilities = json.loads(
        (ROOT.parent / "conformance" / "machine-capabilities.json").read_text("utf-8")
    )
    (ROOT / "capabilities.json").write_bytes(jcs(capabilities))

    fixtures: dict[str, str] = {}

    blank_10 = base_package(contract="typaxis.contract/1.0", master_id="blank-10")
    expected = valid_outcome("paragraph-1.blank-1.0", contract="typaxis.contract/1.0")
    fixtures[expected["fixture_id"]] = write_fixture(
        "profiles/paragraph-1/blank-1.0", jcs(blank_10), expected
    )

    blank_11 = base_package(master_id="blank-11")
    expected = valid_outcome("paragraph-1.blank-1.1", contract="typaxis.contract/1.1")
    fixtures[expected["fixture_id"]] = write_fixture(
        "profiles/paragraph-1/blank-1.1", jcs(blank_11), expected
    )

    combined = combined_package(ttf, ttc)
    expected = valid_outcome("paragraph-1.combined", contract="typaxis.contract/1.1", text=PHRASE)
    expected["advertised_item_coverage"] = ADVERTISED_COVERAGE
    expected["expected"]["side_effects"]["resource_opened"] = True
    expected["resource_hashes"] = [
        {"bytes": len(ttf), "sha256": sha256(ttf), "uri": "fonts/body.ttf"},
        {"bytes": len(ttc), "sha256": sha256(ttc), "uri": "fonts/collection.ttc"},
    ]
    fixtures[expected["fixture_id"]] = write_fixture(
        "profiles/paragraph-1/combined",
        jcs(combined),
        expected,
        sources={"sources/book.json": PHRASE.encode("utf-8")},
        resources={"fonts/body.ttf": ttf, "fonts/collection.ttc": ttc},
    )
    # The package declaration uses the stable book source name from §2.6.
    combined_path = ROOT / "profiles/paragraph-1/combined/job/document-package.json"
    combined_value = json.loads(combined_path.read_bytes())
    combined_value["sources"][0]["uri"] = "sources/book.json"
    combined_path.write_bytes(jcs(combined_value))

    base = base_package(master_id="invalid")
    base_bytes = jcs(base)
    raw_cases = [
        ("p1100-bom", b"\xef\xbb\xbf" + base_bytes, "P1100", "byte:0"),
        ("p1100-nul", b"\0" + base_bytes, "P1100", "byte:0"),
        ("p1100-trailing-token", base_bytes + b" null", "P1100", f"byte:{len(base_bytes) + 1}"),
        ("p1101-malformed-json", b'{"contract":', "P1101", "json:/contract"),
        (
            "p1101-duplicate-escaped-key",
            base_bytes[:1] + b'"c\\u006fntract":"typaxis.contract/1.1",' + base_bytes[1:],
            "P1101",
            "json:/contract",
        ),
    ]
    for name, payload, code, location in raw_cases:
        expected = invalid_outcome(name.replace("-", "."), code, location=location)
        fixtures[expected["fixture_id"]] = write_fixture(f"invalid/{name}", payload, expected)

    typed_mutations: list[tuple[str, dict[str, Any], str]] = []
    candidate = copy.deepcopy(base)
    candidate["unknown"] = 0
    typed_mutations.append(("p1102-unknown-field", candidate, "json:/unknown"))
    candidate = copy.deepcopy(base)
    del candidate["coordinate_unit"]
    typed_mutations.append(("p1102-missing-field", candidate, "json:"))
    candidate = copy.deepcopy(base)
    candidate["sources"][0]["source_id"] = 0.0
    typed_mutations.append(("p1102-float-integer", candidate, "json:/sources/0/source_id"))
    candidate = copy.deepcopy(base)
    candidate["page_masters"]["masters"][0]["width"] = 0
    typed_mutations.append(
        ("p1102-range", candidate, "json:/page_masters/masters/0/width")
    )
    for name, package, location in typed_mutations:
        expected = invalid_outcome(name.replace("-", "."), "P1102", location=location)
        fixtures[expected["fixture_id"]] = write_fixture(f"invalid/{name}", jcs(package), expected)

    candidate = copy.deepcopy(base)
    candidate["contract"] = "typaxis.contract/9.9"
    expected = invalid_outcome("p1103.unknown-contract", "P1103", location="json:/contract")
    expected["contract"] = "typaxis.contract/9.9"
    fixtures[expected["fixture_id"]] = write_fixture(
        "invalid/p1103-unknown-contract", jcs(candidate), expected
    )

    for suffix, delta, succeeds in [("exact", 0, True), ("max-plus-one", -1, False)]:
        candidate = base_package(master_id=f"bytes-{suffix}")
        payload = jcs(candidate)
        limit = len(payload) + delta
        fixture_id = f"i9100.package-bytes-{suffix}"
        args = expectation("unused", fixture_class="limit", exit_code=0, primary_code=None,
            location=None, package_progress="none", sources_progress="none", resources_progress="none",
            effects=side_effects(package=False, source=False, resource=False, layout=False, pdf=False), visible=[])["arguments"]
        args.extend(["--max-document-package-bytes", str(limit)])
        if succeeds:
            expected = valid_outcome(fixture_id, contract="typaxis.contract/1.1")
            expected["fixture_class"] = "limit"
            expected["arguments"] = args
        else:
            expected = invalid_outcome(
                fixture_id, "I9100", exit_code=5, location="json:", package_progress="none",
                package_read=False, fixture_class="limit", arguments=args
            )
        fixtures[fixture_id] = write_fixture(f"invalid/i9100-package-bytes-{suffix}", payload, expected)

    for suffix, delta, succeeds in [("exact", 0, True), ("max-plus-one", -1, False)]:
        candidate = base_package(master_id=f"depth-{suffix}")
        payload = jcs(candidate)
        limit = maximum_json_depth(candidate) + delta
        fixture_id = f"i9101.depth-{suffix}"
        args = expectation("unused", fixture_class="limit", exit_code=0, primary_code=None,
            location=None, package_progress="none", sources_progress="none", resources_progress="none",
            effects=side_effects(package=False, source=False, resource=False, layout=False, pdf=False), visible=[])["arguments"]
        args.extend(["--max-json-nesting-depth", str(limit)])
        if succeeds:
            expected = valid_outcome(fixture_id, contract="typaxis.contract/1.1")
            expected["fixture_class"] = "limit"
            expected["arguments"] = args
        else:
            expected = invalid_outcome(
                fixture_id, "I9101", exit_code=5, location="json:/page_masters/masters/0/body",
                package_progress="raw", fixture_class="limit", arguments=args
            )
        fixtures[fixture_id] = write_fixture(f"invalid/i9101-depth-{suffix}", payload, expected)

    candidate = base_package(master_id="multiple-sources")
    candidate["sources"].append(source_record(b"", "sources/other.json", 1))
    expected = invalid_outcome(
        "p1110.multiple-sources", "P1110", location="json:/sources/0", package_progress="decoded"
    )
    fixtures[expected["fixture_id"]] = write_fixture(
        "invalid/p1110-multiple-sources", jcs(candidate), expected,
        sources={"sources/blank.json": b"", "sources/other.json": b""}
    )
    candidate = base_package(master_id="nonzero-source")
    candidate["sources"] = [source_record(b"", source_id=1)]
    expected = invalid_outcome(
        "p1110.nonzero-entry", "P1110", location="json:/sources/0", package_progress="decoded"
    )
    fixtures[expected["fixture_id"]] = write_fixture(
        "invalid/p1110-nonzero-entry", jcs(candidate), expected
    )
    candidate = base_package(master_id="unsafe-source")
    candidate["sources"][0]["uri"] = "../outside.json"
    expected = invalid_outcome(
        "p1111.unsafe-source", "P1111", location="json:/sources/0", package_progress="decoded"
    )
    fixtures[expected["fixture_id"]] = write_fixture(
        "invalid/p1111-unsafe-source", jcs(candidate), expected
    )

    for suffix, field, value in [
        ("source-length", "utf8_byte_length", 1),
        ("source-hash", "sha256", "0" * 64),
    ]:
        candidate = base_package(master_id=suffix)
        candidate["sources"][0][field] = value
        expected = invalid_outcome(
            f"p1112.{suffix}", "P1112", location="json:/sources/0", package_progress="decoded",
            source_read=True
        )
        fixtures[expected["fixture_id"]] = write_fixture(f"invalid/p1112-{suffix}", jcs(candidate), expected)

    candidate = base_package(source=b"A", master_id="identity-map")
    candidate["text_buffers"] = [
        {
            "text_id": 0,
            "utf8": "B",
            "mappings": [
                {
                    "text_range": {"start_byte": 0, "end_byte": 1},
                    "kind": "identity",
                    "source_span": span(0, 1),
                }
            ],
        }
    ]
    expected = invalid_outcome(
        "p1112.identity-map", "P1112", location="source:0:0-1", package_progress="decoded",
        sources_progress="admitted", source_read=True
    )
    fixtures[expected["fixture_id"]] = write_fixture(
        "invalid/p1112-identity-map", jcs(candidate), expected,
        sources={"sources/blank.json": b"A"}
    )

    for name, code, location, package in [
        (
            "l5100-unsupported-content",
            "L5100",
            "json:/document/blocks/0/children/1",
            rejected_content_package(),
        ),
        (
            "l5101-unsupported-style-master",
            "L5101",
            "json:/style_sheet/rules/0",
            rejected_style_package(),
        ),
        (
            "r7100-unsupported-image",
            "R7100",
            "json:/resources/images/0",
            rejected_image_package(),
        ),
    ]:
        expected = invalid_outcome(
            name.replace("-", "."), code, location=location,
            package_progress="validated", sources_progress="admitted",
            resources_progress="registered", source_read=True
        )
        fixtures[expected["fixture_id"]] = write_fixture(f"invalid/{name}", jcs(package), expected)

    scenario_specs = [
        ("usage.package-outside-root", "negative", 2, None),
        ("i9111.package-symlink", "negative", 3, "I9111"),
        ("i9112.source-symlink", "negative", 3, "I9112"),
        ("i9113.package-mutation", "tamper", 3, "I9113"),
        ("i9113.source-mutation", "tamper", 3, "I9113"),
        ("i9110.host-unavailable", "negative", 3, "I9110"),
        ("usage.unknown-profile", "negative", 2, None),
    ]
    for index, (fixture_id, fixture_class, exit_code, code) in enumerate(scenario_specs):
        package = base_package(master_id=f"scenario-{index}")
        args = None
        if fixture_id == "usage.package-outside-root":
            args = expectation("unused", fixture_class="negative", exit_code=0, primary_code=None,
                location=None, package_progress="none", sources_progress="none", resources_progress="none",
                effects=side_effects(package=False, source=False, resource=False, layout=False, pdf=False), visible=[])["arguments"]
            root_index = args.index("job")
            args[root_index] = "job/root"
        if fixture_id == "usage.unknown-profile":
            args = expectation("unused", fixture_class="negative", exit_code=0, primary_code=None,
                location=None, package_progress="none", sources_progress="none", resources_progress="none",
                effects=side_effects(package=False, source=False, resource=False, layout=False, pdf=False), visible=[])["arguments"]
            args[args.index(PROFILE)] = "typaxis.machine-pdf/unknown"
        expected = expectation(
            fixture_id,
            fixture_class=fixture_class,
            exit_code=exit_code,
            primary_code=code,
            location=None if exit_code == 2 else "global",
            package_progress="none" if code in {None, "I9110", "I9111"} else "decoded",
            sources_progress="none",
            resources_progress="none",
            effects=side_effects(
                package=code not in {None, "I9110", "I9111"},
                source=False,
                resource=False,
                layout=False,
                pdf=False,
            ),
            visible=[] if exit_code == 2 else ["diagnostics", "manifest"],
            arguments=args,
        )
        if fixture_id == "i9113.source-mutation":
            expected["expected"]["side_effects"]["source_read"] = True
        relative = f"scenarios/{fixture_id.replace('.', '-')}"
        fixtures[fixture_id] = write_fixture(relative, jcs(package), expected)
        if fixture_id == "usage.package-outside-root":
            (ROOT / relative / "job/root").mkdir(parents=True, exist_ok=True)

    # Symlinks are deliberately checked-in fixture state on the documented Unix hosts.
    package_symlink = ROOT / "scenarios/i9111-package-symlink/job/document-package.json"
    package_real = package_symlink.with_name("real-package.json")
    package_symlink.rename(package_real)
    package_symlink.symlink_to(package_real.name)
    source_symlink = ROOT / "scenarios/i9112-source-symlink/job/sources/blank.json"
    source_real = source_symlink.with_name("real.json")
    source_symlink.rename(source_real)
    source_symlink.symlink_to(source_real.name)

    diagnostic_package = base_package(master_id="diagnostic-budget")
    diagnostic_package["document"]["blocks"] = [
        {"kind": "page_break", "node_id": node_id, "span": span(), "classes": []}
        for node_id in range(1, 258)
    ]
    expected = invalid_outcome(
        "diagnostics.max-plus-one", "L5100", location="json:/document/blocks/0",
        package_progress="validated", sources_progress="admitted", resources_progress="registered",
        source_read=True, fixture_class="limit"
    )
    fixtures[expected["fixture_id"]] = write_fixture(
        "scenarios/diagnostics-max-plus-one", jcs(diagnostic_package), expected
    )

    for fixture_id, fixture_class, label in [
        ("tamper.receipt-swap", "tamper", "receipt-swap"),
        ("publication.alias-race", "publication", "alias-race"),
        ("publication.partial-failure", "publication", "partial-failure"),
        ("round-trip.canonical", "round_trip", "round-trip"),
    ]:
        package = base_package(master_id=label)
        expected = valid_outcome(fixture_id, contract="typaxis.contract/1.1")
        expected["fixture_class"] = fixture_class
        relative = f"scenarios/{label}"
        fixtures[fixture_id] = write_fixture(relative, jcs(package), expected)
        if fixture_id == "round-trip.canonical":
            (ROOT / relative / "job/input.tsf").write_bytes(b"")
            equivalent = copy.deepcopy(package)
            semantic = copy.deepcopy(package)
            semantic["page_masters"]["masters"][0]["width"] += 1
            (ROOT / relative / "job/equivalent.json").write_text(
                json.dumps(equivalent, ensure_ascii=False, indent=2) + "\n", "utf-8"
            )
            (ROOT / relative / "job/semantic.json").write_bytes(jcs(semantic))

    decoder_rows = [
        (1, "blank_1_1", ["paragraph-1.blank-1.1"]),
        (2, "blank_1_0", ["paragraph-1.blank-1.0"]),
        (3, "combined", ["paragraph-1.combined"]),
        (4, "package_envelope", ["p1100.bom", "p1100.nul", "p1100.trailing.token"]),
        (5, "json_grammar", ["p1101.malformed.json", "p1101.duplicate.escaped.key"]),
        (6, "typed_members", ["p1102.unknown.field", "p1102.missing.field", "p1102.float.integer", "p1102.range"]),
        (7, "unknown_contract", ["p1103.unknown-contract"]),
        (8, "package_bytes", ["i9100.package-bytes-exact", "i9100.package-bytes-max-plus-one"]),
        (9, "json_depth", ["i9101.depth-exact", "i9101.depth-max-plus-one"]),
        (10, "source_profile", ["p1110.multiple-sources", "p1110.nonzero-entry"]),
        (11, "source_path", ["p1111.unsafe-source", "i9112.source-symlink"]),
        (12, "package_root", ["usage.package-outside-root"]),
        (13, "package_open", ["i9111.package-symlink"]),
        (14, "source_identity", ["p1112.source-length", "p1112.source-hash"]),
        (15, "stable_read", ["i9113.package-mutation", "i9113.source-mutation"]),
        (16, "identity_map", ["p1112.identity-map"]),
        (17, "unsupported_content", ["l5100.unsupported.content"]),
        (18, "unsupported_style", ["l5101.unsupported.style.master"]),
        (19, "unsupported_image", ["r7100.unsupported.image"]),
        (20, "host_unavailable", ["i9110.host-unavailable"]),
        (21, "unknown_profile", ["usage.unknown-profile"]),
    ]
    matrix = {
        "contract": "typaxis.machine-fixture-matrix/1",
        "fixtures": [
            {"expected": fixtures[fixture_id], "fixture_id": fixture_id}
            for fixture_id in sorted({item for _, _, ids in decoder_rows for item in ids})
        ],
        "profile": PROFILE,
        "rows": [
            {
                "fixture_ids": ids,
                "id": f"m1-decoder-{row:02d}",
                "test": MATRIX_TEST.format(row=row, name=name),
            }
            for row, name, ids in decoder_rows
        ],
        "verification_commands": [
            "cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine --locked",
            "cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package --locked",
            "python3 schemas/validate.py",
        ],
    }
    (ROOT / "matrices").mkdir(parents=True, exist_ok=True)
    (ROOT / "matrices/m1-paragraph-1.json").write_bytes(jcs(matrix))

    closure_rows = [
        ("receipts", ["tamper.receipt-swap"], "machine_tests::receipt_sessions_are_not_interchangeable"),
        ("diagnostics", ["diagnostics.max-plus-one"], "machine_tests::command_diagnostic_budget_retains_primary_and_omission_note"),
        ("aliases", ["publication.alias-race"], "machine_tests::all_machine_targets_reject_input_aliases"),
        ("publication", ["publication.partial-failure"], "machine_tests::publication_failure_artifact_sets_are_typed"),
        ("round-trip", ["round-trip.canonical"], "machine_tests::canonical_round_trip_relations_hold"),
    ]
    closure_matrix = {
        "contract": "typaxis.machine-fixture-matrix/1",
        "fixtures": [
            {"expected": fixtures[fixture_id], "fixture_id": fixture_id}
            for fixture_id in sorted({item for _, ids, _ in closure_rows for item in ids})
        ],
        "profile": PROFILE,
        "rows": [
            {"fixture_ids": ids, "id": f"m1-closure-{row_id}", "test": test}
            for row_id, ids, test in closure_rows
        ],
        "verification_commands": matrix["verification_commands"],
    }
    (ROOT / "matrices/m1-closure.json").write_bytes(jcs(closure_matrix))


if __name__ == "__main__":
    main()
