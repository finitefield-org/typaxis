#!/usr/bin/env python3
"""Generate the deterministic private MI4-07 book-navigation package."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
JOB = HERE / "job"
SOURCE = b"Chapter link exercise"
FIXED = 65_536


def span(start: int, end: int) -> dict[str, int]:
    return {"end_byte": end, "source_id": 0, "start_byte": start}


package = {
    "contract": "typaxis.contract/1.4",
    "coordinate_unit": "pdf_point_1_65536",
    "document": {
        "blocks": [
            {
                "anchor_id": "part-1",
                "blocks": [
                    {
                        "anchor_id": "chapter-1",
                        "children": [
                            {
                                "kind": "text",
                                "node_id": 3,
                                "span": span(0, 7),
                                "text_span": {
                                    "end_byte": 7,
                                    "start_byte": 0,
                                    "text_id": 0,
                                },
                            }
                        ],
                        "classes": [],
                        "kind": "heading",
                        "language": "FR-latn-fr",
                        "level": 2,
                        "node_id": 2,
                        "span": span(0, 7),
                    },
                    {
                        "children": [
                            {
                                "children": [
                                    {
                                        "kind": "text",
                                        "node_id": 6,
                                        "span": span(8, 12),
                                        "text_span": {
                                            "end_byte": 12,
                                            "start_byte": 8,
                                            "text_id": 0,
                                        },
                                    }
                                ],
                                "kind": "link",
                                "language": "de-DE",
                                "node_id": 5,
                                "span": span(8, 12),
                                "target": {
                                    "anchor_id": "chapter-1",
                                    "kind": "internal",
                                },
                            }
                        ],
                        "classes": [],
                        "kind": "paragraph",
                        "node_id": 4,
                        "span": span(8, 12),
                    },
                    {
                        "anchor_id": "exercise-1",
                        "blocks": [
                            {
                                "children": [
                                    {
                                        "kind": "text",
                                        "node_id": 9,
                                        "span": span(13, 21),
                                        "text_span": {
                                            "end_byte": 21,
                                            "start_byte": 13,
                                            "text_id": 0,
                                        },
                                    }
                                ],
                                "classes": [],
                                "kind": "paragraph",
                                "node_id": 8,
                                "span": span(13, 21),
                            }
                        ],
                        "classes": [],
                        "kind": "semantic_container",
                        "language": "zh-hant-tw",
                        "node_id": 7,
                        "semantic_kind": "exercise",
                        "span": span(13, 21),
                    },
                ],
                "classes": ["book"],
                "kind": "semantic_container",
                "node_id": 1,
                "semantic_kind": "result",
                "span": span(0, 21),
            }
        ],
        "footnotes": [],
        "language": "en-US",
        "node_id": 0,
    },
    "metadata": {
        "author": "Ada Example",
        "created": "2026-08-28T00:00:00Z",
        "identifier": "urn:example:book:1",
        "keywords": ["determinism", "typesetting"],
        "modified": "2026-08-29T00:00:00Z",
        "subject": "Metadata & navigation <proof>",
        "title": "Typaxis Book",
    },
    "outline": {
        "entries": [
            {
                "destination": "part-1",
                "label": "Part I",
                "level": 1,
                "outline_id": 0,
                "parent_outline_id": None,
                "source_kind": "semantic_container",
                "source_node_id": 1,
            },
            {
                "destination": "chapter-1",
                "label": "Chapitre 1",
                "level": 2,
                "outline_id": 1,
                "parent_outline_id": 0,
                "source_kind": "heading",
                "source_node_id": 2,
            },
            {
                "destination": "exercise-1",
                "label": "Exercise 1",
                "level": 2,
                "outline_id": 2,
                "parent_outline_id": 0,
                "source_kind": "semantic_container",
                "source_node_id": 7,
            },
        ]
    },
    "page_masters": {
        "default_master_id": "default",
        "masters": [
            {
                "body": {
                    "height": 600 * FIXED,
                    "width": 800 * FIXED,
                    "x": 100 * FIXED,
                    "y": 100 * FIXED,
                },
                "column_layout": None,
                "footer": None,
                "footer_content": None,
                "footnote": None,
                "header": None,
                "header_content": None,
                "height": 800 * FIXED,
                "master_id": "default",
                "trim": {
                    "height": 800 * FIXED,
                    "width": 1000 * FIXED,
                    "x": 0,
                    "y": 0,
                },
                "width": 1000 * FIXED,
            }
        ],
        "page_progression": "ltr",
        "selection_rules": [],
        "writing_mode": "horizontal-tb",
    },
    "resources": {"font_faces": [], "images": []},
    "sources": [
        {
            "sha256": hashlib.sha256(SOURCE).hexdigest(),
            "source_id": 0,
            "uri": "input.tsf",
            "utf8_byte_length": len(SOURCE),
        }
    ],
    "style_sheet": {
        "rules": [
            {
                "declarations": [
                    {
                        "important": False,
                        "name": "space_before",
                        "value": {"kind": "length", "value": 0},
                    },
                    {
                        "important": False,
                        "name": "space_after",
                        "value": {"kind": "length", "value": 0},
                    },
                    {
                        "important": False,
                        "name": "start_indent",
                        "value": {"kind": "length", "value": 0},
                    },
                    {
                        "important": False,
                        "name": "end_indent",
                        "value": {"kind": "length", "value": 0},
                    },
                    {
                        "important": False,
                        "name": "keep_with_next",
                        "value": {"kind": "boolean", "value": False},
                    },
                ],
                "extends": None,
                "selector": "semantic_container",
                "source_order": 0,
                "style_id": "book-container",
            }
        ]
    },
    "text_buffers": [
        {
            "mappings": [
                {
                    "kind": "identity",
                    "source_span": span(0, len(SOURCE)),
                    "text_range": {"end_byte": len(SOURCE), "start_byte": 0},
                }
            ],
            "text_id": 0,
            "utf8": SOURCE.decode("ascii"),
        }
    ],
}

JOB.mkdir(parents=True, exist_ok=True)
(JOB / "input.tsf").write_bytes(SOURCE)
(JOB / "document-package.json").write_text(
    json.dumps(package, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
    encoding="utf-8",
)

expectation = {
    "destinations": [
        {
            "name": "chapter-1",
            "page_index": 0,
            "view": {"kind": "xyz", "x": 100 * FIXED, "y": 100 * FIXED},
        },
        {
            "name": "exercise-1",
            "page_index": 1,
            "view": {"kind": "xyz", "x": 100 * FIXED, "y": 100 * FIXED},
        },
        {
            "name": "part-1",
            "page_index": 0,
            "view": {"kind": "xyz", "x": 0, "y": 0},
        },
    ],
    "document_language": "en-US",
    "engine": {"name": "typaxis", "version": "0.1.0"},
    "language_paints": [
        {
            "actual_text": None,
            "language": "fr-Latn-FR",
            "page_index": 0,
        },
        {"actual_text": None, "language": "de-DE", "page_index": 0},
        {
            "actual_text": None,
            "language": "zh-Hant-TW",
            "page_index": 1,
        },
    ],
    "links": [
        {
            "destination": "chapter-1",
            "page_index": 0,
            "rect": [100 * FIXED, 130 * FIXED, 160 * FIXED, 150 * FIXED],
        }
    ],
    "metadata": package["metadata"],
    "outline": [
        {
            "destination": item["destination"],
            "label": item["label"],
            "level": item["level"],
            "outline_id": item["outline_id"],
            "parent_outline_id": item["parent_outline_id"],
            "source_node_id": item["source_node_id"],
        }
        for item in package["outline"]["entries"]
    ],
    "pages": [
        {"height": 800 * FIXED, "page_index": 0, "width": 1000 * FIXED},
        {"height": 800 * FIXED, "page_index": 1, "width": 1000 * FIXED},
    ],
}
(HERE / "pdf-expectation.json").write_text(
    json.dumps(expectation, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
    encoding="utf-8",
)
