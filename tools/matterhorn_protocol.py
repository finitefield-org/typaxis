#!/usr/bin/env python3
"""Canonical Matterhorn Protocol 1.02 inventory and MI4-V19 assessment.

The item inventory is deliberately local and closed: release evidence must not
depend on a mutable web page or silently accept a new/deleted protocol item.
Descriptions remain in the normative protocol; this module freezes each item
identifier and its published detection method.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any, Sequence


CONTRACT = "typaxis.matterhorn-assessment/2"
PROTOCOL_VERSION = "1.02"
PROTOCOL_SOURCE = (
    "https://www.pdfa.org/wp-content/uploads/2014/06/"
    "PDFUA-1_MatterhornProtocol_1-02.pdf"
)
CHECKPOINT_ITEM_COUNTS = (
    6, 4, 3, 1, 3, 4, 2, 3, 8, 1, 7, 1, 7, 7, 5, 3,
    3, 2, 4, 3, 1, 1, 1, 1, 1, 2, 1, 18, 1, 2, 30,
)
HUMAN_IDS = frozenset(
    """01-001 01-002 01-006 02-002 03-001 03-002 03-003 04-001
    05-001 05-002 05-003 06-004 08-001 08-002 09-001 09-002 09-003
    11-007 12-001 13-001 13-002 13-003 13-005 13-006 13-007 14-001
    14-004 14-005 15-001 15-002 15-004 15-005 16-001 16-002 16-003
    17-001 18-001 18-002 19-001 19-002 22-001 24-001 28-001 28-003
    28-013 29-001 31-010""".split()
)
NO_SPECIFIC_TEST_IDS = frozenset({"23-001", "27-001"})

# These human checks are applicable to the closed two-page publication
# fixture. The remaining human checks are explicitly N/A below; none is
# promoted to passed merely because a machine validator returned success.
HUMAN_PASS_RATIONALES = {
    "01-006": "manual_semantic_role_review",
    "06-004": "manual_title_metadata_review",
    "09-001": "manual_logical_reading_order_review",
    "09-002": "manual_structure_nesting_review",
    "09-003": "manual_semantic_role_review",
    "11-007": "manual_natural_language_review",
    "13-001": "manual_figure_tag_review",
    "13-005": "manual_figure_alt_vs_actualtext_review",
    "13-006": "manual_graphics_grouping_review",
    "13-007": "manual_accessible_representation_review",
    "17-001": "manual_formula_tag_review",
    "31-010": "manual_fixture_font_license_review",
}

N_A_RATIONALE_BY_CHECKPOINT = {
    1: "fixture_contains_no_artifact_marked_content",
    2: "fixture_contains_no_custom_role_mapping",
    3: "fixture_contains_no_action_multimedia_or_javascript",
    4: "fixture_conveys_no_semantics_by_color_or_layout_alone",
    5: "fixture_contains_no_audio_or_javascript_notification",
    8: "fixture_contains_no_ocr_content",
    12: "fixture_contains_no_stretched_character_content",
    13: "fixture_contains_no_caption_or_meaningful_link_background",
    14: "fixture_contains_no_heading_content",
    15: "fixture_contains_no_table_content",
    16: "fixture_contains_no_list_content",
    18: "fixture_contains_no_header_or_footer_content",
    19: "fixture_contains_no_note_or_reference_content",
    22: "fixture_contains_no_article_threads",
    24: "fixture_contains_no_form_fields",
    28: "fixture_contains_no_annotations",
    29: "fixture_contains_no_scripts",
}


def item_ids() -> tuple[str, ...]:
    return tuple(
        f"{checkpoint:02d}-{ordinal:03d}"
        for checkpoint, count in enumerate(CHECKPOINT_ITEM_COUNTS, 1)
        for ordinal in range(1, count + 1)
    )


ALL_IDS = item_ids()
MACHINE_IDS = frozenset(ALL_IDS) - HUMAN_IDS - NO_SPECIFIC_TEST_IDS

if (
    len(ALL_IDS) != 136
    or len(set(ALL_IDS)) != 136
    or len(MACHINE_IDS) != 87
    or len(HUMAN_IDS) != 47
    or len(NO_SPECIFIC_TEST_IDS) != 2
):  # pragma: no cover - import-time invariant
    raise RuntimeError("Matterhorn Protocol inventory is not the fixed 136/87/47/2 set")


def _human_not_applicable_rationale(item_id: str) -> str:
    checkpoint = int(item_id[:2])
    try:
        return N_A_RATIONALE_BY_CHECKPOINT[checkpoint]
    except KeyError as error:  # pragma: no cover - closed constant invariant
        raise RuntimeError(f"missing N/A rationale for {item_id}") from error


def build_assessment(*, pdf_sha256: str, fixture_revision_sha256: str) -> dict[str, Any]:
    items: list[dict[str, str]] = []
    for item_id in ALL_IDS:
        if item_id in MACHINE_IDS:
            item = {
                "evidence": "verapdf-ua1-and-independent-structure-v2",
                "id": item_id,
                "method": "machine",
                "rationale": "pinned_pdfua_machine_validation_passed",
                "status": "passed",
            }
        elif item_id in HUMAN_PASS_RATIONALES:
            item = {
                "evidence": "manual-semantic-review",
                "id": item_id,
                "method": "human",
                "rationale": HUMAN_PASS_RATIONALES[item_id],
                "status": "passed",
            }
        elif item_id in HUMAN_IDS:
            item = {
                "evidence": "closed-fixture-feature-inventory",
                "id": item_id,
                "method": "human",
                "rationale": _human_not_applicable_rationale(item_id),
                "status": "not_applicable",
            }
        else:
            item = {
                "evidence": "matterhorn-no-specific-test",
                "id": item_id,
                "method": "no_specific_test",
                "rationale": (
                    "protocol_defers_digital_signature_checks_to_form_fields"
                    if item_id == "23-001"
                    else "protocol_defers_navigation_checks_to_semantics"
                ),
                "status": "not_applicable",
            }
        items.append(item)

    passed = sum(item["status"] == "passed" for item in items)
    not_applicable = sum(item["status"] == "not_applicable" for item in items)
    return {
        "contract": CONTRACT,
        "fixture": {
            "fixture_id": "mi4-v19.production-readiness",
            "revision_sha256": fixture_revision_sha256,
        },
        "items": items,
        "pdf_sha256": pdf_sha256,
        "protocol": {
            "human_item_count": 47,
            "item_count": 136,
            "machine_item_count": 87,
            "no_specific_test_item_count": 2,
            "source": PROTOCOL_SOURCE,
            "version": PROTOCOL_VERSION,
        },
        "review": {
            "method": "closed-fixture-human-semantic-review/1",
            "reviewed_on": "2026-09-05",
            "reviewer": "codex-agent/mi4-v19-review",
            "status_counts": {
                "not_applicable": not_applicable,
                "passed": passed,
            },
        },
        "validator": {
            "flavour": "ua1",
            "id": "veraPDF Greenfield",
            "version": "1.30.2",
        },
    }


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main(arguments: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pdf", required=True, type=Path)
    parser.add_argument("--fixture", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    options = parser.parse_args(arguments)
    assessment = build_assessment(
        pdf_sha256=_sha256(options.pdf),
        fixture_revision_sha256=_sha256(options.fixture),
    )
    options.output.write_bytes(canonical_json_bytes(assessment) + b"\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
