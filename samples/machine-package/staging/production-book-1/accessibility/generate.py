#!/usr/bin/env python3
"""Generate the deterministic private MI4-09 accessibility package."""

from __future__ import annotations

import hashlib
import json
import runpy
from pathlib import Path


HERE = Path(__file__).resolve().parent
MACHINE_ROOT = HERE.parents[2]
helpers = runpy.run_path(str(MACHINE_ROOT / "generate.py"))
ttf = helpers["synthetic_ascii_ttf"](include_math=True)
ttc = helpers["single_face_ttc"](ttf)
png = bytes.fromhex(
    (MACHINE_ROOT / "staging/basic-document-1/machine-figure/job/figure.data.hex").read_text("ascii")
)
package = helpers["table_document_combined_package"](ttf, ttc, png)
fixed = 65_536


def inserted_buffer(value: str) -> int:
    text_id = len(package["text_buffers"])
    package["text_buffers"].append(
        {
            "mappings": [
                {
                    "kind": "inserted",
                    "source_span": None,
                    "text_range": {
                        "end_byte": len(value.encode("utf-8")),
                        "start_byte": 0,
                    },
                }
            ],
            "text_id": text_id,
            "utf8": value,
        }
    )
    return text_id


def text_node(value: str) -> dict:
    text_id = inserted_buffer(value)
    return {
        "kind": "text",
        "node_id": 0,
        "span": helpers["span"](),
        "text_span": {
            "end_byte": len(value.encode("utf-8")),
            "start_byte": 0,
            "text_id": text_id,
        },
    }


package["contract"] = "typaxis.contract/1.4"
package["metadata"] = {
    "author": "Typaxis",
    "created": "2026-08-29T00:00:00Z",
    "identifier": "urn:typaxis:fixture:accessibility",
    "keywords": ["accessibility", "tagged PDF"],
    "modified": "2026-08-29T00:00:00Z",
    "subject": "Deterministic tagged structure",
    "title": "Typaxis Accessibility Fixture",
}
package["document"]["language"] = "en-US"

master = package["page_masters"]["masters"][0]
master["trim"] = {
    "height": master["height"],
    "width": master["width"],
    "x": 0,
    "y": 0,
}
master["header_content"] = None
master["footer_content"] = None
master["column_layout"] = None
helpers["add_footnote_frame"](package)
package["page_masters"]["page_progression"] = "ltr"
package["page_masters"]["writing_mode"] = "horizontal-tb"


def advanced_blocks(blocks: list[dict]) -> None:
    for block in blocks:
        kind = block["kind"]
        if kind == "figure":
            block["placement"] = "block"
            advanced_blocks(block["caption"])
        elif kind == "list":
            for item in block["items"]:
                advanced_blocks(item["blocks"])
        elif kind == "table":
            for row in [*block["head"], *block["body"]]:
                for cell in row["cells"]:
                    advanced_blocks(cell["blocks"])
        elif kind == "semantic_container":
            advanced_blocks(block["blocks"])


advanced_blocks(package["document"]["blocks"])
for face in package["resources"]["font_faces"]:
    face["media_type"] = (
        "ttc-truetype-glyf" if face["uri"] == "collection.ttc" else "sfnt-truetype-glyf"
    )
for image in package["resources"]["images"]:
    image["media_type"] = "png"

# The heading itself owns the destination in the 1.4 navigation contract.
heading = package["document"]["blocks"][0]
heading["anchor_id"] = "top"
heading["children"] = [child for child in heading["children"] if child["kind"] != "anchor"]
# The MI4-09 fixture needs one Link annotation whose selected rectangle and
# destination are already owned by the navigation receipt. URI annotations
# remain outside this private slice.
package["document"]["blocks"][2]["children"] = package["document"]["blocks"][2]["children"][:1]

math_text_id = inserted_buffer("x^2x+1")
package["text_buffers"][math_text_id]["mappings"] = [
    {
        "kind": "identity",
        "source_span": {"end_byte": 3, "source_id": 0, "start_byte": 0},
        "text_range": {"end_byte": 3, "start_byte": 0},
    },
    {
        "kind": "identity",
        "source_span": {"end_byte": 6, "source_id": 0, "start_byte": 3},
        "text_range": {"end_byte": 6, "start_byte": 3},
    },
]
math_paragraph = {
    "children": [
        {
            "kind": "inline_math",
            "math_source": {
                "language": "typaxis-math",
                "text_span": {"end_byte": 3, "start_byte": 0, "text_id": math_text_id},
                "version": "1",
            },
            "node_id": 0,
            "span": helpers["span"](0, 3),
            "speech": "x squared",
        }
    ],
    "classes": [],
    "kind": "paragraph",
    "node_id": 0,
    "span": helpers["span"](0, 3),
}
display_math = {
    "classes": ["equation"],
    "kind": "display_math",
    "math_source": {
        "language": "typaxis-math",
        "text_span": {"end_byte": 6, "start_byte": 3, "text_id": math_text_id},
        "version": "1",
    },
    "node_id": 0,
    "span": helpers["span"](3, 6),
    "speech": "x plus one",
}
container = {
    "anchor_id": "exercise",
    "blocks": [
        {
            "children": [text_node("Accessible exercise")],
            "classes": [],
            "kind": "paragraph",
            "node_id": 0,
            "span": helpers["span"](),
        }
    ],
    "classes": ["accessible"],
    "kind": "semantic_container",
    "node_id": 0,
    "semantic_kind": "exercise",
    "span": helpers["span"](),
}
container["span"] = helpers["span"](6, 6)
container["blocks"][0]["span"] = helpers["span"](6, 6)
container["blocks"][0]["children"][0]["span"] = helpers["span"](6, 6)


def trailing_text_node(value: str) -> dict:
    node = text_node(value)
    node["span"] = helpers["span"](6, 6)
    return node


heading_coverage = [
    {
        "anchor_id": None,
        "children": [trailing_text_node(f"Heading level {level}")],
        "classes": [],
        "kind": "heading",
        "level": level,
        "node_id": 0,
        "span": helpers["span"](6, 6),
    }
    for level in range(2, 7)
]
inline_role_coverage = {
    "children": [
        {
            "children": [trailing_text_node("emphasized")],
            "kind": "emphasis",
            "node_id": 0,
            "span": helpers["span"](6, 6),
        },
        {
            "children": [trailing_text_node("strong")],
            "kind": "strong",
            "node_id": 0,
            "span": helpers["span"](6, 6),
        },
    ],
    "classes": [],
    "kind": "paragraph",
    "node_id": 0,
    "span": helpers["span"](6, 6),
}
unordered_list = {
    "classes": [],
    "items": [
        {
            "blocks": [
                {
                    "children": [trailing_text_node("Unordered item")],
                    "classes": [],
                    "kind": "paragraph",
                    "node_id": 0,
                    "span": helpers["span"](6, 6),
                }
            ],
            "node_id": 0,
            "span": helpers["span"](6, 6),
        }
    ],
    "kind": "list",
    "node_id": 0,
    "ordered": False,
    "span": helpers["span"](6, 6),
    "start": None,
}


def semantic_container(kind: str) -> dict:
    return {
        "anchor_id": None,
        "blocks": [
            {
                "children": [trailing_text_node(f"Accessible {kind}")],
                "classes": [],
                "kind": "paragraph",
                "node_id": 0,
                "span": helpers["span"](6, 6),
            }
        ],
        "classes": ["accessible"],
        "kind": "semantic_container",
        "node_id": 0,
        "semantic_kind": kind,
        "span": helpers["span"](6, 6),
    }


package["document"]["blocks"].extend(
    [
        math_paragraph,
        display_math,
        *heading_coverage,
        inline_role_coverage,
        unordered_list,
        semantic_container("result"),
        semantic_container("proof"),
        container,
    ]
)

# One reference and one definition exercise Note/Reference closure.
reference_paragraph = package["document"]["blocks"][2]
reference_paragraph["children"].append(
    {
        "footnote_id": "note-1",
        "kind": "footnote_reference",
        "node_id": 0,
        "span": helpers["span"](),
    }
)
package["document"]["footnotes"] = [
    {
        "blocks": [
            {
                "children": [text_node("Accessible footnote")],
                "classes": [],
                "kind": "paragraph",
                "node_id": 0,
                "span": helpers["span"](),
            }
        ],
        "footnote_id": "note-1",
        "node_id": 0,
        "span": helpers["span"](),
    }
]

package["style_sheet"]["rules"].extend(
    [
        {
            "declarations": [
                {"important": False, "name": "font_family", "value": {"families": ["Body"], "kind": "font_family_list"}},
                {"important": False, "name": "font_size", "value": {"kind": "length", "value": 12 * fixed}},
                {"important": False, "name": "line_height", "value": {"kind": "length", "value": 16 * fixed}},
                {"important": False, "name": "space_before", "value": {"kind": "length", "value": 2 * fixed}},
                {"important": False, "name": "space_after", "value": {"kind": "length", "value": 2 * fixed}},
                {"important": False, "name": "start_indent", "value": {"kind": "length", "value": 0}},
                {"important": False, "name": "end_indent", "value": {"kind": "length", "value": 0}},
                {"important": False, "name": "keep_with_next", "value": {"kind": "boolean", "value": False}},
            ],
            "extends": None,
            "selector": "semantic_container",
            "source_order": len(package["style_sheet"]["rules"]),
            "style_id": "accessible-container",
        },
        {
            "declarations": [
                {"important": False, "name": "font_family", "value": {"families": ["Body"], "kind": "font_family_list"}},
                {"important": False, "name": "font_size", "value": {"kind": "length", "value": 12 * fixed}},
                {"important": False, "name": "line_height", "value": {"kind": "length", "value": 16 * fixed}},
                {"important": False, "name": "space_before", "value": {"kind": "length", "value": 2 * fixed}},
                {"important": False, "name": "space_after", "value": {"kind": "length", "value": 2 * fixed}},
                {"important": False, "name": "start_indent", "value": {"kind": "length", "value": 0}},
                {"important": False, "name": "end_indent", "value": {"kind": "length", "value": 0}},
                {"important": False, "name": "keep_with_next", "value": {"kind": "boolean", "value": False}},
                {"important": False, "name": "text_align", "value": {"kind": "keyword", "value": "center"}},
            ],
            "extends": None,
            "selector": "display_math",
            "source_order": len(package["style_sheet"]["rules"]) + 1,
            "style_id": "accessible-display-math",
        },
    ]
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
        elif kind == "figure":
            for child in value["caption"]:
                block(child)
        elif kind == "semantic_container":
            for child in value["blocks"]:
                block(child)

    for value in package["document"]["blocks"]:
        block(value)
    for value in package["document"]["footnotes"]:
        issue(value)
        for child in value["blocks"]:
            block(child)


renumber()
heading = package["document"]["blocks"][0]
container = next(
    block
    for block in package["document"]["blocks"]
    if block.get("anchor_id") == "exercise"
)
package["outline"] = {
    "entries": [
        {
            "destination": "top",
            "label": "Typaxis Accessibility Fixture",
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
            "source_node_id": container["node_id"],
        },
    ]
}

job = HERE / "job"
job.mkdir(parents=True, exist_ok=True)
source = b"x^2x+1"
package_path = job / "document-package.json"
package["sources"][0]["sha256"] = hashlib.sha256(source).hexdigest()
package["sources"][0]["utf8_byte_length"] = len(source)
package_path.write_text(
    json.dumps(package, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
    encoding="utf-8",
)
(job / "input.tsf").write_bytes(source)
(job / "body.ttf").write_bytes(ttf)
(job / "collection.ttc").write_bytes(ttc)
(job / "figure.data").write_bytes(png)
