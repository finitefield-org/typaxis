#!/usr/bin/env python3
"""Generate the public, lossless MI4-13 production-book fixture."""

from __future__ import annotations

import copy
import hashlib
import json
import runpy
from collections import Counter
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[4]
STAGING = REPOSITORY / "samples/machine-package/staging/production-book-1"
ACCESSIBILITY = STAGING / "accessibility"

# The focused accessibility generator is the frozen source of the ordinary M4
# nodes.  Copy its in-memory value and then add every producer-composed vector
# relation and every remaining declared media kind.
namespace = runpy.run_path(str(ACCESSIBILITY / "generate.py"))
package = copy.deepcopy(namespace["package"])
fixed = 65_536
source = b"x^2x+1s1s2x+yf1j1v1v2x+yAB"


def span(start: int, end: int) -> dict:
    return {"end_byte": end, "source_id": 0, "start_byte": start}


def inserted_text(value: str, at: int = 0) -> dict:
    text_id = len(package["text_buffers"])
    encoded_length = len(value.encode("utf-8"))
    package["text_buffers"].append(
        {
            "mappings": [
                {
                    "kind": "inserted",
                    "source_span": None,
                    "text_range": {"end_byte": encoded_length, "start_byte": 0},
                }
            ],
            "text_id": text_id,
            "utf8": value,
        }
    )
    return {
        "kind": "text",
        "node_id": 0,
        "span": span(at, at),
        "text_span": {"end_byte": encoded_length, "start_byte": 0, "text_id": text_id},
    }


source_text_id = len(package["text_buffers"])
package["text_buffers"].append(
    {
        "mappings": [
            {
                "kind": "identity",
                "source_span": span(start, end),
                "text_range": {"end_byte": end, "start_byte": start},
            }
            for start, end in ((10, 13), (21, 24), (24, 26))
        ],
        "text_id": source_text_id,
        "utf8": source.decode("ascii"),
    }
)

inline_metrics = {
    "advance": 2_031_616,
    "ascent": 655_360,
    "baseline": 589_824,
    "descent": 196_608,
    "origin_x": 0,
    "viewport": {"height": 786_432, "width": 1_966_080},
}
safe1_inline_metrics = {
    **inline_metrics,
    "advance": 1_376_256,
    "ascent": 589_824,
    "baseline": 524_288,
    "descent": 131_072,
    "viewport": {"height": 655_360, "width": 1_310_720},
}
spacing = {"after": 16_384, "before": 16_384}

vector_container = {
    "anchor_id": "vectors",
    "blocks": [
        {
            "children": [
                {
                    "actual_text": "safe vector one",
                    "alt": "SafeVector 1 inline",
                    "image_id": 1,
                    "kind": "inline_vector",
                    "metrics": copy.deepcopy(safe1_inline_metrics),
                    "node_id": 0,
                    "spacing": copy.deepcopy(spacing),
                    "span": span(6, 8),
                },
                {
                    "actual_text": "safe vector two",
                    "alt": "SafeVector 2 inline",
                    "image_id": 2,
                    "kind": "inline_vector",
                    "metrics": copy.deepcopy(inline_metrics),
                    "node_id": 0,
                    "spacing": copy.deepcopy(spacing),
                    "span": span(8, 10),
                },
                {
                    "actual_text": None,
                    "alt": "x plus y",
                    "image_id": 2,
                    "kind": "math_vector",
                    "metrics": copy.deepcopy(inline_metrics),
                    "node_id": 0,
                    "source_tex": {
                        "text_span": {"end_byte": 13, "start_byte": 10, "text_id": source_text_id}
                    },
                    "spacing": copy.deepcopy(spacing),
                    "span": span(10, 13),
                },
            ],
            "classes": [],
            "kind": "paragraph",
            "node_id": 0,
            "span": span(6, 13),
        },
        {
            "alt": "SafeVector 1 figure",
            "caption": [
                {
                    "children": [inserted_text("SafeVector 1 caption", 15)],
                    "classes": [],
                    "kind": "paragraph",
                    "node_id": 0,
                    "span": span(15, 15),
                }
            ],
            "classes": [],
            "image_id": 1,
            "kind": "figure",
            "node_id": 0,
            "placement": "block",
            "span": span(13, 15),
        },
        {
            "alt": "JPEG figure",
            "caption": [
                {
                    "children": [inserted_text("JPEG caption", 17)],
                    "classes": [],
                    "kind": "paragraph",
                    "node_id": 0,
                    "span": span(17, 17),
                }
            ],
            "classes": [],
            "image_id": 3,
            "kind": "figure",
            "node_id": 0,
            "placement": "block",
            "span": span(15, 17),
        },
        {
            "alt": "SafeVector 1 vector figure",
            "caption": [],
            "classes": [],
            "image_id": 1,
            "kind": "vector_figure",
            "node_id": 0,
            "span": span(17, 19),
            "viewport": {"height": 2_621_440, "width": 5_242_880},
        },
        {
            "alt": "SafeVector 2 vector figure",
            "caption": [],
            "classes": [],
            "image_id": 2,
            "kind": "vector_figure",
            "node_id": 0,
            "span": span(19, 21),
            "viewport": {"height": 786_432, "width": 1_966_080},
        },
        {
            "actual_text": None,
            "alt": "x plus y, equation AB",
            "classes": [],
            "equation_number": {
                "minimum_gap": fixed,
                "node_id": 0,
                "span": span(24, 26),
                "text_span": {"end_byte": 26, "start_byte": 24, "text_id": source_text_id},
            },
            "image_id": 2,
            "kind": "math_vector_block",
            "metrics": copy.deepcopy(inline_metrics),
            "node_id": 0,
            "source_tex": {
                "text_span": {"end_byte": 24, "start_byte": 21, "text_id": source_text_id}
            },
            "span": span(21, 26),
        },
    ],
    "classes": ["production-vectors"],
    "kind": "semantic_container",
    "node_id": 0,
    "semantic_kind": "result",
    "span": span(6, 26),
}
package["document"]["blocks"].append(vector_container)

# Native inline math uses the TrueType face, native display math uses the TTC,
# and the producer equation number uses the CFF face.  Consequently every
# advertised font container is selected by real PDF paint.
for rule in package["style_sheet"]["rules"]:
    if rule["selector"] == "display_math":
        for declaration in rule["declarations"]:
            if declaration["name"] == "font_family":
                declaration["value"]["families"] = ["Collection"]


def append_rule(style_id: str, selector: str, declarations: list[dict]) -> None:
    package["style_sheet"]["rules"].append(
        {
            "declarations": declarations,
            "extends": None,
            "selector": selector,
            "source_order": len(package["style_sheet"]["rules"]),
            "style_id": style_id,
        }
    )


append_rule(
    "production-vector-figure",
    "vector_figure",
    [
        {"important": False, "name": "page", "value": {"kind": "keyword", "value": "auto"}},
        {"important": False, "name": "space_before", "value": {"kind": "length", "value": fixed}},
        {"important": False, "name": "space_after", "value": {"kind": "length", "value": fixed}},
        {"important": False, "name": "start_indent", "value": {"kind": "length", "value": 0}},
        {"important": False, "name": "end_indent", "value": {"kind": "length", "value": 0}},
        {"important": False, "name": "text_align", "value": {"kind": "keyword", "value": "center"}},
        {"important": False, "name": "keep_with_next", "value": {"kind": "boolean", "value": False}},
        {"important": False, "name": "keep_caption", "value": {"kind": "boolean", "value": True}},
    ],
)
append_rule(
    "production-math-vector-block",
    "math_vector_block",
    [
        {"important": False, "name": "page", "value": {"kind": "keyword", "value": "auto"}},
        {"important": False, "name": "space_before", "value": {"kind": "length", "value": fixed}},
        {"important": False, "name": "space_after", "value": {"kind": "length", "value": fixed}},
        {"important": False, "name": "start_indent", "value": {"kind": "length", "value": 0}},
        {"important": False, "name": "end_indent", "value": {"kind": "length", "value": 0}},
        {"important": False, "name": "text_align", "value": {"kind": "keyword", "value": "center"}},
        {"important": False, "name": "keep_with_next", "value": {"kind": "boolean", "value": False}},
    ],
)
append_rule(
    "equation-number-text",
    "semantic_container",
    [
        {
            "important": False,
            "name": "font_family",
            "value": {"families": ["Typaxis CFF Fixture"], "kind": "font_family_list"},
        },
        {"important": False, "name": "font_size", "value": {"kind": "length", "value": 12 * fixed}},
        {"important": False, "name": "line_height", "value": {"kind": "length", "value": 14 * fixed}},
    ],
)


def renumber() -> None:
    next_id = 1

    def issue(value: dict) -> None:
        nonlocal next_id
        value["node_id"] = next_id
        next_id += 1

    def inline(value: dict) -> None:
        issue(value)
        for child in value.get("children", []):
            inline(child)

    def block(value: dict) -> None:
        issue(value)
        kind = value["kind"]
        if kind in {"paragraph", "heading"}:
            for child in value["children"]:
                inline(child)
        elif kind == "list":
            for item in value["items"]:
                issue(item)
                for child in item["blocks"]:
                    block(child)
        elif kind == "table":
            for row in [*value["head"], *value["body"]]:
                issue(row)
                for cell in row["cells"]:
                    issue(cell)
                    for child in cell["blocks"]:
                        block(child)
        elif kind in {"figure", "vector_figure"}:
            for child in value["caption"]:
                block(child)
        elif kind == "semantic_container":
            for child in value["blocks"]:
                block(child)
        elif kind == "math_vector_block" and value["equation_number"] is not None:
            issue(value["equation_number"])

    for value in package["document"]["blocks"]:
        block(value)
    for value in package["document"]["footnotes"]:
        issue(value)
        for child in value["blocks"]:
            block(child)


renumber()
heading = package["document"]["blocks"][0]
exercise = next(
    block for block in package["document"]["blocks"] if block.get("anchor_id") == "exercise"
)
vectors = next(
    block for block in package["document"]["blocks"] if block.get("anchor_id") == "vectors"
)
package["outline"] = {
    "entries": [
        {
            "destination": "top",
            "label": "Typaxis Production Book Fixture",
            "level": 1,
            "outline_id": 0,
            "parent_outline_id": None,
            "source_kind": "heading",
            "source_node_id": heading["node_id"],
        },
        {
            "destination": "exercise",
            "label": "Accessible exercise",
            "level": 2,
            "outline_id": 1,
            "parent_outline_id": 0,
            "source_kind": "semantic_container",
            "source_node_id": exercise["node_id"],
        },
        {
            "destination": "vectors",
            "label": "Producer vectors",
            "level": 2,
            "outline_id": 2,
            "parent_outline_id": 0,
            "source_kind": "semantic_container",
            "source_node_id": vectors["node_id"],
        },
    ]
}
package["metadata"] = {
    "author": "Typaxis",
    "created": "2026-09-05T00:00:00Z",
    "identifier": "urn:typaxis:fixture:production-book-1",
    "keywords": ["production-book", "vector"],
    "modified": "2026-09-05T00:00:00Z",
    "subject": "Atomic M4 production profile publication",
    "title": "Typaxis Production Book Fixture",
}

job = HERE / "job"
(job / "svg").mkdir(parents=True, exist_ok=True)
body_ttf = (ACCESSIBILITY / "job/body.ttf").read_bytes()
collection_ttc = (ACCESSIBILITY / "job/collection.ttc").read_bytes()
png = (ACCESSIBILITY / "job/figure.data").read_bytes()
safe1 = (STAGING / "vector-media/job/art.vector").read_bytes()
safe2 = (STAGING / "precomposed-vector/svg/x-plus-y.svg").read_bytes()
jpeg = bytes.fromhex((STAGING / "jpeg-media/color-2x1.jpg.hex").read_text("ascii"))
cff = bytes.fromhex((STAGING / "cff-media/typaxis-cff-fixture.otf.hex").read_text("ascii"))

package["resources"] = {
    "font_faces": [
        {
            "expected_sha256": hashlib.sha256(body_ttf).hexdigest(),
            "face_index": 0,
            "family": "Body",
            "font_face_id": 0,
            "media_type": "sfnt-truetype-glyf",
            "uri": "body.ttf",
        },
        {
            "expected_sha256": hashlib.sha256(collection_ttc).hexdigest(),
            "face_index": 0,
            "family": "Collection",
            "font_face_id": 1,
            "media_type": "ttc-truetype-glyf",
            "uri": "collection.ttc",
        },
        {
            "expected_sha256": hashlib.sha256(cff).hexdigest(),
            "face_index": 0,
            "family": "Typaxis CFF Fixture",
            "font_face_id": 2,
            "media_type": "sfnt-cff1",
            "uri": "typaxis-cff-fixture.otf",
        },
    ],
    "images": [
        {
            "expected_sha256": hashlib.sha256(png).hexdigest(),
            "image_id": 0,
            "media_type": "png",
            "uri": "figure.data",
        },
        {
            "expected_sha256": hashlib.sha256(safe1).hexdigest(),
            "image_id": 1,
            "media_type": "svg-safe-1",
            "uri": "art.vector",
        },
        {
            "expected_sha256": hashlib.sha256(safe2).hexdigest(),
            "image_id": 2,
            "media_type": "svg-safe-2",
            "uri": "svg/x-plus-y.svg",
            "vector_provenance": {
                "engine_id": "vmb.texToSvg",
                "engine_version": "2026.09.0",
                "rules_version": "vmb.math-safe-svg/1",
            },
        },
        {
            "expected_sha256": hashlib.sha256(jpeg).hexdigest(),
            "image_id": 3,
            "media_type": "jpeg-baseline",
            "uri": "color-2x1.jpg",
        },
    ],
}
package["sources"] = [
    {
        "sha256": hashlib.sha256(source).hexdigest(),
        "source_id": 0,
        "uri": "input.tsf",
        "utf8_byte_length": len(source),
    }
]


def preorder() -> list[dict]:
    output: list[dict] = [{"kind": "document", "node_id": 0}]

    def visit_inline(value: dict) -> None:
        output.append(value)
        for child in value.get("children", []):
            visit_inline(child)

    def visit_block(value: dict) -> None:
        output.append(value)
        kind = value["kind"]
        if kind in {"paragraph", "heading"}:
            for child in value["children"]:
                visit_inline(child)
        elif kind == "list":
            for item in value["items"]:
                output.append({**item, "kind": "list_item"})
                for child in item["blocks"]:
                    visit_block(child)
        elif kind == "table":
            for section, rows in (("head", value["head"]), ("body", value["body"])):
                for row in rows:
                    output.append({**row, "kind": f"table_{section}_row"})
                    for cell in row["cells"]:
                        output.append({**cell, "kind": f"table_{section}_cell"})
                        for child in cell["blocks"]:
                            visit_block(child)
        elif kind in {"figure", "vector_figure"}:
            for child in value["caption"]:
                visit_block(child)
        elif kind == "semantic_container":
            for child in value["blocks"]:
                visit_block(child)
        elif kind == "math_vector_block" and value["equation_number"] is not None:
            output.append({**value["equation_number"], "kind": "equation_number"})

    for value in package["document"]["blocks"]:
        visit_block(value)
    for value in package["document"]["footnotes"]:
        output.append({**value, "kind": "footnote_definition"})
        for child in value["blocks"]:
            visit_block(child)
    return output


nodes = preorder()
math_sources = []
for node in nodes:
    source_value = node.get("math_source") or node.get("source_tex")
    if source_value is not None:
        math_sources.append(
            {
                "kind": node["kind"],
                "node_id": node["node_id"],
                "source_span": source_value["text_span"],
                "syntax_span": node["span"],
            }
        )
ledger = {
    "contract": "typaxis.production-book-source-ledger/1",
    "math_sources": math_sources,
    "node_count": len(nodes),
    "node_kind_counts": dict(sorted(Counter(node["kind"] for node in nodes).items())),
    "outline": package["outline"]["entries"],
    "reading_order": [node["node_id"] for node in nodes],
    "resources": {
        "font_face_ids": [value["font_face_id"] for value in package["resources"]["font_faces"]],
        "image_ids": [value["image_id"] for value in package["resources"]["images"]],
    },
    "source_sha256": hashlib.sha256(source).hexdigest(),
}

publication = json.loads(
    (STAGING / "publication-expectation.json").read_text("utf-8")
)
expected = {
    "advertised_item_coverage": publication["advertised_item_coverage"],
    "arguments": [
        "job/document-package.json",
        "-o",
        "$OUTPUT/output.pdf",
        "--package-root",
        "job",
        "--profile",
        "typaxis.machine-pdf/production-book-1",
        "--resource-root",
        "job",
        "--trace",
        "$OUTPUT/trace.json",
        "--trace-text",
        "--no-compress",
        "--emit-build-manifest",
        "$OUTPUT/manifest.json",
        "--emit-diagnostics",
        "$OUTPUT/diagnostics.json",
    ],
    "command": "build-package",
    "contract": "typaxis.contract/1.4",
    "expected": {
        "exit_code": 0,
        "location": None,
        "manifest_progress": {
            "package": "validated",
            "resources": "admitted",
            "sources": "admitted",
        },
        "normalized_extracted_text": None,
        "page_count": 2,
        "primary_code": None,
        "side_effects": {
            "layout_started": True,
            "package_read": True,
            "pdf_started": True,
            "resource_opened": True,
            "source_read": True,
        },
        "visible_artifacts": ["diagnostics", "manifest", "pdf", "trace"],
    },
    "fixture_class": "positive",
    "fixture_id": "production-book-1.combined",
    "package": "job/document-package.json",
    "profile": "typaxis.machine-pdf/production-book-1",
    "resource_hashes": [],
}
expected["resource_hashes"] = [
    {
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
        "uri": resource["uri"],
    }
    for resource, payload in (
        *zip(package["resources"]["font_faces"], (body_ttf, collection_ttc, cff)),
        *zip(package["resources"]["images"], (png, safe1, safe2, jpeg)),
    )
]
expected["expected"]["normalized_extracted_text"] = (
    "x squared x plus one Basic document page top internal 1 1 Accessible footnote "
    "1. First item 2. Second entry PNG caption Header A Header B alpha beta gamma "
    "delta Heading level 2 Heading level 3 Heading level 4 Heading level 5 Heading "
    "level 6 emphasized strong • Unordered item Accessible result Accessible proof "
    "Accessible exercise safe vector one safe vector two x plus y SafeVector 1 caption "
    "JPEG caption AB x plus y, equation AB"
)

canonical = lambda value: json.dumps(
    value, ensure_ascii=False, separators=(",", ":"), sort_keys=True
).encode("utf-8")
(job / "document-package.json").write_bytes(canonical(package))
(HERE / "ledger.json").write_bytes(canonical(ledger))
(HERE / "expected.json").write_bytes(canonical(expected))
(job / "input.tsf").write_bytes(source)
(job / "body.ttf").write_bytes(body_ttf)
(job / "collection.ttc").write_bytes(collection_ttc)
(job / "typaxis-cff-fixture.otf").write_bytes(cff)
(job / "figure.data").write_bytes(png)
(job / "art.vector").write_bytes(safe1)
(job / "svg/x-plus-y.svg").write_bytes(safe2)
(job / "color-2x1.jpg").write_bytes(jpeg)
