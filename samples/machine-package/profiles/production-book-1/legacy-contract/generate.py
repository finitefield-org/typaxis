#!/usr/bin/env python3
"""Generate the MI4-13 old-contract/production-profile rejection fixture."""

from __future__ import annotations

import json
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[4]
SOURCE = (
    REPOSITORY
    / "samples/machine-package/profiles/basic-document-1/combined/job"
)
JOB = HERE / "job"
JOB.mkdir(parents=True, exist_ok=True)

package = json.loads((SOURCE / "document-package.json").read_text("utf-8"))
expected = {
    "advertised_item_coverage": [],
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
        "--no-compress",
        "--emit-build-manifest",
        "$OUTPUT/manifest.json",
        "--emit-diagnostics",
        "$OUTPUT/diagnostics.json",
    ],
    "command": "build-package",
    "contract": package["contract"],
    "expected": {
        "exit_code": 1,
        "location": "json:",
        "manifest_progress": {
            "package": "validated",
            "resources": "none",
            "sources": "admitted",
        },
        "normalized_extracted_text": None,
        "page_count": None,
        "primary_code": "L5100",
        "side_effects": {
            "layout_started": False,
            "package_read": True,
            "pdf_started": False,
            "resource_opened": False,
            "source_read": True,
        },
        "visible_artifacts": ["diagnostics", "manifest"],
    },
    "fixture_class": "tamper",
    "fixture_id": "production-book-1.legacy-contract",
    "package": "job/document-package.json",
    "profile": "typaxis.machine-pdf/production-book-1",
    "resource_hashes": [],
}


def canonical(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")


(JOB / "document-package.json").write_bytes(canonical(package))
(JOB / "input.tsf").write_bytes((SOURCE / "input.tsf").read_bytes())
(HERE / "expected.json").write_bytes(canonical(expected))
