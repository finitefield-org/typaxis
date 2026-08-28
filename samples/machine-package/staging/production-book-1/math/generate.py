#!/usr/bin/env python3
"""Generate the deterministic private MI4-05 math package fixture."""

from __future__ import annotations

import copy
import hashlib
import json
import runpy
from pathlib import Path


HERE = Path(__file__).resolve().parent
MACHINE_ROOT = HERE.parents[2]
helpers = runpy.run_path(str(MACHINE_ROOT / "generate.py"))
font = helpers["synthetic_ascii_ttf"](include_math=True)
source = b"x^{2}x+1"
fixed = 65_536


def span(start: int, end: int) -> dict[str, int]:
    return {"end_byte": end, "source_id": 0, "start_byte": start}


package = {
    "contract": "typaxis.contract/1.4",
    "coordinate_unit": "pdf_point_1_65536",
    "metadata": {
        "author": None,
        "created": None,
        "identifier": None,
        "keywords": [],
        "modified": None,
        "subject": None,
        "title": None,
    },
    "outline": {"entries": []},
    "document": {
        "blocks": [
            {
                "blocks": [
                    {
                        "children": [
                            {
                                "kind": "inline_math",
                                "math_source": {
                                    "language": "typaxis-math",
                                    "text_span": {
                                        "end_byte": 5,
                                        "start_byte": 0,
                                        "text_id": 0,
                                    },
                                    "version": "1",
                                },
                                "node_id": 3,
                                "span": span(0, 5),
                                "speech": "x squared",
                            }
                        ],
                        "classes": [],
                        "kind": "paragraph",
                        "node_id": 2,
                        "span": span(0, 5),
                    },
                    {
                        "classes": ["equation"],
                        "kind": "display_math",
                        "math_source": {
                            "language": "typaxis-math",
                            "text_span": {
                                "end_byte": 8,
                                "start_byte": 5,
                                "text_id": 0,
                            },
                            "version": "1",
                        },
                        "node_id": 4,
                        "span": span(5, 8),
                        "speech": "x plus one",
                    },
                ],
                "classes": ["math-section"],
                "kind": "semantic_container",
                "node_id": 1,
                "semantic_kind": "result",
                "anchor_id": None,
                "span": span(0, 8),
            }
        ],
        "footnotes": [],
        "language": "und",
        "node_id": 0,
    },
    "page_masters": {
        "default_master_id": "default",
        "masters": [
            {
                "body": {
                    "height": 100 * fixed,
                    "width": 200 * fixed,
                    "x": 20 * fixed,
                    "y": 20 * fixed,
                },
                "column_layout": None,
                "footer": None,
                "footer_content": None,
                "footnote": None,
                "header": None,
                "header_content": None,
                "height": 140 * fixed,
                "master_id": "default",
                "trim": {
                    "height": 140 * fixed,
                    "width": 240 * fixed,
                    "x": 0,
                    "y": 0,
                },
                "width": 240 * fixed,
            }
        ],
        "page_progression": "ltr",
        "selection_rules": [],
        "writing_mode": "horizontal-tb",
    },
    "resources": {
        "font_faces": [
            {
                "expected_sha256": hashlib.sha256(font).hexdigest(),
                "face_index": 0,
                "family": "Math",
                "font_face_id": 0,
                "media_type": "sfnt-truetype-glyf",
                "uri": "math.ttf",
            }
        ],
        "images": [],
    },
    "sources": [
        {
            "sha256": hashlib.sha256(source).hexdigest(),
            "source_id": 0,
            "uri": "input.tsf",
            "utf8_byte_length": len(source),
        }
    ],
    "style_sheet": {
        "rules": [
            {
                "declarations": [
                    {
                        "important": False,
                        "name": "font_family",
                        "value": {"families": ["Math"], "kind": "font_family_list"},
                    },
                    {
                        "important": False,
                        "name": "font_size",
                        "value": {"kind": "length", "value": 12 * fixed},
                    },
                    {
                        "important": False,
                        "name": "line_height",
                        "value": {"kind": "length", "value": 16 * fixed},
                    },
                    {
                        "important": False,
                        "name": "space_before",
                        "value": {"kind": "length", "value": 2 * fixed},
                    },
                    {
                        "important": False,
                        "name": "space_after",
                        "value": {"kind": "length", "value": 2 * fixed},
                    },
                    {
                        "important": False,
                        "name": "keep_with_next",
                        "value": {"kind": "boolean", "value": True},
                    },
                ],
                "extends": None,
                "selector": "semantic_container",
                "source_order": 0,
                "style_id": "math-base",
            },
            {
                "declarations": [
                    {
                        "important": False,
                        "name": "text_align",
                        "value": {"kind": "keyword", "value": "center"},
                    },
                    {
                        "important": False,
                        "name": "start_indent",
                        "value": {"kind": "length", "value": 4 * fixed},
                    },
                    {
                        "important": False,
                        "name": "end_indent",
                        "value": {"kind": "length", "value": 4 * fixed},
                    },
                ],
                "extends": "math-base",
                "selector": "display_math.equation",
                "source_order": 1,
                "style_id": "display-equation",
            },
        ]
    },
    "text_buffers": [
        {
            "mappings": [
                {
                    "kind": "identity",
                    "source_span": span(0, 5),
                    "text_range": {"end_byte": 5, "start_byte": 0},
                },
                {
                    "kind": "identity",
                    "source_span": span(5, 8),
                    "text_range": {"end_byte": 8, "start_byte": 5},
                },
            ],
            "text_id": 0,
            "utf8": source.decode("ascii"),
        }
    ],
}

job = HERE / "job"
job.mkdir(parents=True, exist_ok=True)
(job / "document-package.json").write_text(
    json.dumps(package, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
    encoding="utf-8",
)
page_package = copy.deepcopy(package)
page_package["page_masters"]["masters"][0]["body"]["height"] = 18 * fixed
(job / "page-document-package.json").write_text(
    json.dumps(page_package, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
    encoding="utf-8",
)
keep_package = copy.deepcopy(package)
keep_source = source + b"y"
keep_package["sources"][0]["uri"] = "keep-input.tsf"
keep_package["sources"][0]["utf8_byte_length"] = len(keep_source)
keep_package["sources"][0]["sha256"] = hashlib.sha256(keep_source).hexdigest()
keep_package["text_buffers"][0]["utf8"] = keep_source.decode("ascii")
keep_package["text_buffers"][0]["mappings"].append(
    {
        "kind": "identity",
        "source_span": span(8, 9),
        "text_range": {"end_byte": 9, "start_byte": 8},
    }
)
keep_package["document"]["blocks"][0]["span"] = span(0, 9)
keep_package["document"]["blocks"][0]["blocks"].append(
    {
        "classes": ["equation"],
        "kind": "display_math",
        "math_source": {
            "language": "typaxis-math",
            "text_span": {"end_byte": 9, "start_byte": 8, "text_id": 0},
            "version": "1",
        },
        "node_id": 5,
        "span": span(8, 9),
        "speech": "y",
    }
)
keep_package["page_masters"]["masters"][0]["body"]["height"] = 48 * fixed
(job / "keep-document-package.json").write_text(
    json.dumps(keep_package, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
    encoding="utf-8",
)
(job / "input.tsf").write_bytes(source)
(job / "keep-input.tsf").write_bytes(keep_source)
(job / "math.ttf").write_bytes(font)
