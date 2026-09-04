#!/usr/bin/env python3
"""Independently verify private precomposed-vector artifacts and release evidence.

The verifier consumes an output directory produced by the crate-private
MI4-V18 runner and closes MI4-V19's external publication-readiness gate. It
never writes into the checked-in sample corpus. In normal mode it checks
canonical sidecars, cross-layer receipt relations, PDF vector
operators/accessibility text, the combined corpus ledger, and negative-case
coverage. It can also verify the complete production proof set, emit canonical
per-host evidence, or aggregate evidence from the required hosts.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
from typing import Any, Iterable

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

try:
    from tools import verify_pdf_structure as pdf_structure
    from tools import verify_pdf_differential as pdf_differential
    from tools import matterhorn_protocol
except ModuleNotFoundError:  # Direct `python3 tools/...` execution.
    import verify_pdf_structure as pdf_structure
    import verify_pdf_differential as pdf_differential
    import matterhorn_protocol


VERIFIER_ID = "typaxis.verify-precomposed-vector/2"
VERIFIER_VERSION = "2"
EXTERNAL_COMMAND_TIMEOUT_SECONDS = 120
EVIDENCE_CONTRACT = "typaxis.machine-precomposed-vector-evidence/2"
ARTIFACT_CONTRACT = "typaxis.private-precomposed-vector-artifacts/1"
FIXTURE_ID = "mi4-v18.precomposed-vector-combined"
PUBLICATION_FIXTURE_ID = "mi4-v19.production-readiness"
PRODUCTION_READINESS_CONTRACT = "typaxis.production-book-resource-set-receipt/2"
PRODUCTION_PROFILE = "typaxis.machine-pdf/production-book-1"
PRODUCTION_COMPONENTS = [
    "typaxis.resource-profile/png/1",
    "typaxis.resource-profile/safe-vector/2",
    "typaxis.resource-profile/jpeg-baseline/1",
    "typaxis.resource-profile/truetype-glyf/1",
    "typaxis.resource-profile/sfnt-cff1/1",
]
PRODUCTION_IMAGE_MEDIA = ["png", "svg-safe-1", "svg-safe-2", "jpeg-baseline"]
PRODUCTION_FONT_MEDIA = [
    "sfnt-truetype-glyf",
    "ttc-truetype-glyf",
    "sfnt-cff1",
]
PRODUCTION_PROOF_FILES = {
    "accessibility-manifest.json",
    "accessibility.pdf",
    "cff-manifest.json",
    "cff.pdf",
    "jpeg-manifest.json",
    "jpeg.pdf",
    "math-manifest.json",
    "math.pdf",
    "navigation-manifest.json",
    "navigation.pdf",
    "safe-vector-1-manifest.json",
    "safe-vector-1.pdf",
    "safe-vector-2-manifest.json",
    "safe-vector-2.pdf",
    "semantic-manifest.json",
    "semantic.pdf",
}
PRODUCTION_RESOURCE_PATTERNS = (
    "accessibility/job/*",
    "book-navigation/job/*",
    "cff-media/*.hex",
    "cff-media/job/*",
    "jpeg-media/*.hex",
    "jpeg-media/job/*",
    "math/job/*",
    "precomposed-vector/*.tsv",
    "precomposed-vector/document-package*.json",
    "precomposed-vector/fragments/*",
    "precomposed-vector/input.tsf",
    "precomposed-vector/svg/*",
    "precomposed-vector/tex/*",
    "semantic-container/job/*",
    "vector-media/job/*",
)
PRODUCTION_MANIFEST_ALGORITHMS = {
    "accessibility-manifest.json": "typaxis.tagged-pdf-manifest/1",
    "cff-manifest.json": "typaxis.cff1-manifest/1",
    "jpeg-manifest.json": "typaxis.jpeg-manifest/1",
    "math-manifest.json": "typaxis.math-manifest/1",
    "navigation-manifest.json": "typaxis.book-navigation-manifest/1",
    "safe-vector-1-manifest.json": "typaxis.safe-vector-manifest/1",
    "semantic-manifest.json": "typaxis.semantic-container-manifest/1",
}
PRODUCTION_DIRECT_PDF_HASH_MANIFESTS = {
    "cff-manifest.json",
    "jpeg-manifest.json",
    "math-manifest.json",
    "safe-vector-1-manifest.json",
}
EXPECTED_ARTIFACTS = {
    "block-layout-trace.json",
    "book-navigation-manifest.json",
    "build-manifest-vector.json",
    "corpus-admission.json",
    "corpus-display.json",
    "corpus-output.pdf",
    "dedupe-ten-use.pdf",
    "dedupe-two-alias.pdf",
    "display-v2.json",
    "effective-document-package.json",
    "figure-build-manifest-vector.json",
    "figure-output.pdf",
    "inline-layout-trace.json",
    "math-vector-manifest.json",
    "output.pdf",
    "pdf-observation.json",
    "phase-receipts.json",
    "safe-vector-manifest.json",
    "tagged-pdf-expectation.json",
    "tagged-pdf-manifest.json",
    "verification.json",
}
EXPECTED_PHASES = [
    "wire",
    "syntax-metrics-source-language",
    "profile-style",
    "resource-admission",
    "metric-math-binding",
    "inline-block-layout",
    "display-navigation",
    "content-form-plan",
    "structure-marked-content",
    "final-tagged-pdf-observations",
    "manifests",
]
REQUIRED_CASES = {
    "aligned-block",
    "fraction-equality",
    "generic-block-inherit",
    "generic-block-override",
    "generic-inline-inherit",
    "generic-inline-override",
    "integral",
    "large-brackets",
    "long-block",
    "matrix",
    "not-divides",
    "numbered-aligned",
    "ordered-pair",
    "scripts",
    "similar",
    "sum",
    "x-plus-y",
    "x-plus-y-alias",
}
REQUIRED_FRAGMENTS = {
    "block-number",
    "block-page-end",
    "dedupe-aliases",
    "dedupe-ten-use",
    "japanese-boundaries",
    "language-kinds",
    "line-end",
    "mixed-heights",
}
REQUIRED_CASE_CATEGORIES = {
    "aligned-block": {"aligned", "language-math-block-inherit"},
    "fraction-equality": {"fill-opacity", "fraction-equality", "stroke-opacity"},
    "generic-block-inherit": {"language-generic-block-inherit"},
    "generic-block-override": {"language-generic-block-override"},
    "generic-inline-inherit": {"language-generic-inline-inherit"},
    "generic-inline-override": {"language-generic-inline-override"},
    "integral": {"integral"},
    "large-brackets": {"large-brackets"},
    "long-block": {"language-math-block-override", "long-block"},
    "matrix": {"clip", "matrix"},
    "not-divides": {"not-divides", "stroke"},
    "numbered-aligned": {"equation-number"},
    "ordered-pair": {"ordered-pair"},
    "scripts": {"subscript", "superscript"},
    "similar": {"actual-text-authored", "language-math-inline-override", "similar"},
    "sum": {"sum"},
    "x-plus-y": {
        "actual-text-alt-fallback",
        "current-color",
        "language-math-inline-inherit",
        "x-plus-y",
    },
    "x-plus-y-alias": {"same-content-alias"},
}
REQUIRED_FRAGMENT_CATEGORIES = {
    "block-number": {"block-math", "equation-number"},
    "block-page-end": {"block-math", "page-end"},
    "dedupe-aliases": {"cross-id-alias", "dedupe"},
    "dedupe-ten-use": {"dedupe", "ten-use"},
    "japanese-boundaries": {"brackets", "japanese", "punctuation"},
    "language-kinds": {"language-inheritance", "language-override"},
    "line-end": {"inline-math", "line-end"},
    "mixed-heights": {"inline-math", "mixed-heights"},
}
REQUIRED_ASSERTIONS = {
    "PV-15.1-ALIGNED",
    "PV-15.1-ALTERNATIVE",
    "PV-15.1-BASIC",
    "PV-15.1-BLOCKS",
    "PV-15.1-LANGUAGE",
    "PV-15.1-MIXED-HEIGHTS",
    "PV-15.1-PAINT",
    "PV-15.1-REUSE",
    "PV-15.2-BASELINE",
    "PV-15.2-BLOCK-ALIGN",
    "PV-15.2-BLOCK-ATOMIC",
    "PV-15.2-INLINE-ATOMIC",
    "PV-15.2-LINE-HEIGHT",
    "PV-15.2-SPACING",
    "PV-15.3-ACCESSIBILITY",
    "PV-15.3-DEDUP",
    "PV-15.3-EXTRACTION",
    "PV-15.3-MANIFEST",
    "PV-15.3-NAVIGATION",
    "PV-15.3-VECTOR",
    "PV-15.4-ADMISSION",
    "PV-15.4-LAYOUT",
    "PV-15.4-RECEIPTS",
    "PV-15.4-SYNTAX",
    "PV-15.5-ALIASED-PATH",
    "PV-15.5-SCHEDULE",
}
REQUIRED_ASSERTION_CHECKS = {
    "PV-15.1-ALIGNED": "corpus-category-closure",
    "PV-15.1-ALTERNATIVE": "actual-text-resolution",
    "PV-15.1-BASIC": "corpus-category-closure",
    "PV-15.1-BLOCKS": "block-atomicity",
    "PV-15.1-LANGUAGE": "language-four-kind-closure",
    "PV-15.1-MIXED-HEIGHTS": "inline-line-metrics",
    "PV-15.1-PAINT": "vector-paint-closure",
    "PV-15.1-REUSE": "dedupe-resource-closure",
    "PV-15.2-BASELINE": "inline-baseline-equation",
    "PV-15.2-BLOCK-ALIGN": "block-alignment-number",
    "PV-15.2-BLOCK-ATOMIC": "block-page-end-whole-move",
    "PV-15.2-INLINE-ATOMIC": "inline-atomic-break",
    "PV-15.2-LINE-HEIGHT": "inline-max-ascent-descent",
    "PV-15.2-SPACING": "inline-boundary-spacing",
    "PV-15.3-ACCESSIBILITY": "tagged-vector-semantics",
    "PV-15.3-DEDUP": "one-form-ten-do-and-cross-id-provenance",
    "PV-15.3-EXTRACTION": "actual-text-document-order",
    "PV-15.3-MANIFEST": "manifest-dependency-closure",
    "PV-15.3-NAVIGATION": "language-navigation-closure",
    "PV-15.3-VECTOR": "no-raster-no-tex-no-form-mcid",
    "PV-15.4-ADMISSION": "negative-resource-admission",
    "PV-15.4-LAYOUT": "negative-layout-limits",
    "PV-15.4-RECEIPTS": "negative-receipt-tamper",
    "PV-15.4-SYNTAX": "negative-syntax-contract",
    "PV-15.5-ALIASED-PATH": "private-staging-path-alias",
    "PV-15.5-SCHEDULE": "owner-private-schedule-byte-identity",
}
REQUIRED_ASSERTION_REFERENCES = {
    "PV-15.1-ALIGNED": "cases.tsv#aligned-block",
    "PV-15.1-ALTERNATIVE": "cases.tsv#similar,x-plus-y",
    "PV-15.1-BASIC": "cases.tsv#x-plus-y,similar,not-divides,ordered-pair",
    "PV-15.1-BLOCKS": "cases.tsv#aligned-block,long-block,numbered-aligned",
    "PV-15.1-LANGUAGE": "fragments.tsv#language-kinds",
    "PV-15.1-MIXED-HEIGHTS": "fragments.tsv#mixed-heights",
    "PV-15.1-PAINT": "cases.tsv#fraction-equality,matrix,not-divides,x-plus-y",
    "PV-15.1-REUSE": "dedupe-ten-use.pdf,dedupe-two-alias.pdf",
    "PV-15.2-BASELINE": "inline-layout-trace.json",
    "PV-15.2-BLOCK-ALIGN": "block-layout-trace.json",
    "PV-15.2-BLOCK-ATOMIC": "fragments.tsv#block-page-end",
    "PV-15.2-INLINE-ATOMIC": "fragments.tsv#line-end",
    "PV-15.2-LINE-HEIGHT": "fragments.tsv#mixed-heights",
    "PV-15.2-SPACING": "fragments.tsv#japanese-boundaries,line-end",
    "PV-15.3-ACCESSIBILITY": "tagged-pdf-manifest.json,corpus-output.pdf",
    "PV-15.3-DEDUP": "dedupe-ten-use.pdf,dedupe-two-alias.pdf",
    "PV-15.3-EXTRACTION": "output.pdf,corpus-output.pdf",
    "PV-15.3-MANIFEST": "build-manifest-vector.json,effective-document-package.json",
    "PV-15.3-NAVIGATION": "book-navigation-manifest.json",
    "PV-15.3-VECTOR": "output.pdf,corpus-output.pdf",
    "PV-15.4-ADMISSION": "negative-integration.tsv",
    "PV-15.4-LAYOUT": "negative-integration.tsv",
    "PV-15.4-RECEIPTS": "negative-integration.tsv",
    "PV-15.4-SYNTAX": "negative-integration.tsv",
    "PV-15.5-ALIASED-PATH": "verify_reproducibility.py",
    "PV-15.5-SCHEDULE": "artifact-index.json",
}
REQUIRED_NEGATIVE_OUTCOMES = {
    "alternative-control": ("syntax", "P1102"),
    "ast-depth-max-plus-one": ("syntax", "P1121"),
    "ast-node-max-plus-one": ("syntax", "P1120"),
    "block-height-max-plus-one": ("layout", "L5100"),
    "block-width-max-plus-one": ("layout", "L5100"),
    "clip-alpha": ("resource", "R7100"),
    "content-key-tamper": ("content-form", "I9190"),
    "equation-number-collision": ("layout", "L5100"),
    "external-image": ("resource", "R7100"),
    "forbidden-script": ("resource", "R7100"),
    "form-object-tamper": ("pdf", "I9190"),
    "fragment-max-plus-one": ("layout", "L5110"),
    "hash-mismatch": ("resource", "R7100"),
    "inline-height-max-plus-one": ("layout", "L5100"),
    "inline-visual-overhang": ("layout", "L5100"),
    "invalid-alpha": ("resource", "R7100"),
    "ir-allocation-max-plus-one": ("resource", "R7111"),
    "language-owner-tamper": ("structure", "I9190"),
    "malformed-provenance": ("profile", "P1102"),
    "malformed-unclosed": ("resource", "R7100"),
    "manifest-link-tamper": ("manifest", "I9190"),
    "metric-baseline-outside": ("syntax", "P1102"),
    "metric-missing": ("syntax", "P1102"),
    "native-flow-swap": ("binding", "I9190"),
    "nonuniform-scale": ("binding", "P1102"),
    "old-profile-rejection": ("profile", "P1102"),
    "pdf-object-max-plus-one": ("pdf", "G6100"),
    "resource-conflict": ("resource", "R7100"),
    "source-span-invalid": ("syntax", "P1102"),
    "structure-owner-tamper": ("structure", "I9190"),
    "style-registry-swap": ("style", "I9190"),
    "text-aggregate-max-plus-one": ("syntax", "T2101"),
    "text-buffer-max-plus-one": ("syntax", "T2100"),
    "unsupported-text": ("resource", "R7100"),
    "use-observation-tamper": ("pdf", "I9190"),
    "vector-depth-max-plus-one": ("resource", "R7122"),
    "vector-node-max-plus-one": ("resource", "R7120"),
    "vector-path-max-plus-one": ("resource", "R7121"),
}
EXPECTED_COVERAGE = [
    "block:math_vector_block",
    "block:vector_figure",
    "inline:inline_vector",
    "inline:math_vector",
    "vector_format:svg-safe-2",
    "vector_metric:advance",
    "vector_metric:ascent",
    "vector_metric:baseline",
    "vector_metric:descent",
    "vector_metric:origin_x",
    "vector_metric:viewport",
]
REQUIRED_CHECKS = {
    "artifact_index",
    "block_atomicity",
    "corpus_coverage",
    "corpus_admission",
    "corpus_pdf",
    "dedupe_alias",
    "dedupe_ten_use",
    "display_manifest_pdf_closure",
    "effective_package_identity",
    "inline_baseline",
    "inline_line_metrics",
    "language_structure",
    "legacy_figure_closure",
    "manifest_dependency",
    "negative_fixture_coverage",
    "pdf_accessible_text",
    "pdf_structure_v2",
    "pdf_vector_only",
    "phase_order",
    "public_surface_isolation",
    "resource_hashes",
}
EXTERNAL_REQUIRED_CHECKS = {
    "external_mupdf_multi_dpi",
    "external_poppler_page_text",
    "external_vector_operator",
    "matterhorn_1_02",
    "production_resource_set_v2",
    "publication_capability_projection",
    "verapdf_ua1",
}
RELEASE_REQUIRED_CHECKS = REQUIRED_CHECKS | EXTERNAL_REQUIRED_CHECKS
_HEX64 = re.compile(r"^[0-9a-f]{64}$")
_OBJECT = re.compile(rb"(?m)^(\d+) 0 obj\n(.*?)\nendobj\n", re.DOTALL)
_ACTUAL_TEXT = re.compile(rb"/ActualText <([0-9A-F]+)>")
_MARKED_CONTENT = re.compile(
    rb"/(Formula|Figure|Span) << /ActualText <([0-9A-F]+)> /Lang <([0-9A-F]+)> >> BDC"
)
_DO = re.compile(rb"/V[0-9]+ Do(?:\r?\n|\s)")
_RASTER_IMAGE = re.compile(rb"/Subtype\s*/Image(?:\s|/|>|\[)|/ImageMask(?:\s|/|>|\[)")
_STARTXREF = re.compile(rb"\nstartxref\n(0|[1-9][0-9]*)\n%%EOF\n\Z")
_UNICODE_WHITE_SPACE = {
    0x0009,
    0x000A,
    0x000B,
    0x000C,
    0x000D,
    0x0020,
    0x0085,
    0x00A0,
    0x1680,
    *range(0x2000, 0x200B),
    0x2028,
    0x2029,
    0x202F,
    0x205F,
    0x3000,
}


class PrecomposedVectorError(ValueError):
    """The generated artifact set is incomplete, unsafe, or inconsistent."""


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def canonical_json_bytes(value: Any) -> bytes:
    stack = [value]
    while stack:
        current = stack.pop()
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                raise PrecomposedVectorError("canonical JSON contains an unpaired surrogate")
        elif current is None or isinstance(current, bool):
            continue
        elif isinstance(current, int):
            if not -(2**53 - 1) <= current <= 2**53 - 1:
                raise PrecomposedVectorError("canonical JSON integer is not exactly representable")
        elif isinstance(current, list):
            stack.extend(current)
        elif isinstance(current, dict):
            if not all(isinstance(key, str) for key in current):
                raise PrecomposedVectorError("canonical JSON object key is not a string")
            stack.extend(current.keys())
            stack.extend(current.values())
        else:
            raise PrecomposedVectorError(
                f"canonical JSON contains unsupported {type(current).__name__}"
            )
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def _load_json(path: Path, *, canonical: bool = True) -> Any:
    def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise PrecomposedVectorError(f"{path}: duplicate JSON member {key!r}")
            result[key] = value
        return result

    try:
        payload = path.read_bytes()
        if not payload.endswith(b"\n") or b"\r" in payload or payload.startswith(b"\xef\xbb\xbf"):
            raise PrecomposedVectorError(f"{path}: JSON must be UTF-8, LF, and final-LF canonical")
        value = json.loads(payload, object_pairs_hook=no_duplicates)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PrecomposedVectorError(f"cannot read JSON {path}: {error}") from error
    if canonical and payload != canonical_json_bytes(value) + b"\n":
        raise PrecomposedVectorError(f"{path}: JSON bytes are not canonical JCS")
    return value


def _exact_object(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise PrecomposedVectorError(
            f"{label}: keys differ (missing={sorted(keys - actual)}, extra={sorted(actual - keys)})"
        )
    return value


def _integer(value: Any, label: str, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < minimum:
        raise PrecomposedVectorError(f"{label}: expected integer >= {minimum}")
    return value


def _hash(value: Any, label: str) -> str:
    if not isinstance(value, str) or not _HEX64.fullmatch(value):
        raise PrecomposedVectorError(f"{label}: expected lowercase SHA-256")
    return value


def _read_tsv(path: Path, header: list[str]) -> list[dict[str, str]]:
    try:
        payload = path.read_bytes()
        text = payload.decode("utf-8")
    except (OSError, UnicodeError) as error:
        raise PrecomposedVectorError(f"cannot read {path}: {error}") from error
    if not payload.endswith(b"\n") or b"\r" in payload or payload.startswith(b"\xef\xbb\xbf"):
        raise PrecomposedVectorError(f"{path}: TSV must use UTF-8, LF, and final LF")
    lines = text[:-1].split("\n")
    if not lines or lines[0].split("\t") != header:
        raise PrecomposedVectorError(f"{path}: unexpected TSV header")
    rows: list[dict[str, str]] = []
    for line_number, line in enumerate(lines[1:], 2):
        fields = line.split("\t")
        if len(fields) != len(header) or any(not field for field in fields):
            raise PrecomposedVectorError(f"{path}:{line_number}: invalid TSV row")
        rows.append(dict(zip(header, fields, strict=True)))
    if not rows:
        raise PrecomposedVectorError(f"{path}: empty TSV")
    return rows


def _nonignored_rust_test_names(repository: Path) -> set[str]:
    names: set[str] = set()
    for source in sorted((repository / "workspace/crates").rglob("*.rs")):
        attributes: list[str] = []
        for line in source.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith("#[") and stripped.endswith("]"):
                attributes.append(stripped)
                continue
            match = re.fullmatch(r"fn\s+([a-z][a-z0-9_]*)\s*\([^)]*\)\s*(?:->.*)?\s*\{", stripped)
            if match is not None and "#[test]" in attributes and not any(
                attribute.startswith("#[ignore") for attribute in attributes
            ):
                names.add(match.group(1))
            if stripped:
                attributes.clear()
    return names


def _tsv_integer(value: str, label: str, *, minimum: int | None = None) -> int:
    if not re.fullmatch(r"-?(?:0|[1-9][0-9]*)", value):
        raise PrecomposedVectorError(f"{label}: noncanonical fixed-point integer")
    parsed = int(value)
    if not -(2**63) <= parsed <= 2**63 - 1:
        raise PrecomposedVectorError(f"{label}: fixed-point integer exceeds i64")
    if minimum is not None and parsed < minimum:
        raise PrecomposedVectorError(f"{label}: expected integer >= {minimum}")
    return parsed


def _accessible_text(value: str, label: str) -> str:
    scalars = [ord(character) for character in value]
    if not any(scalar not in _UNICODE_WHITE_SPACE for scalar in scalars) or any(
        scalar < 0x20 or 0x7F <= scalar <= 0x9F for scalar in scalars
    ):
        raise PrecomposedVectorError(f"{label}: text is empty or contains a control")
    return value


def _rect_as_list(value: Any, label: str) -> list[int]:
    rect = _exact_object(value, {"height", "width", "x", "y"}, label)
    result = []
    for field in ("x", "y", "width", "height"):
        minimum = 1 if field in {"width", "height"} else 0
        result.append(_integer(rect[field], f"{label}.{field}", minimum))
    return result


def _matrix_as_list(value: Any, label: str) -> list[int]:
    matrix = _exact_object(
        value,
        {"a_16_16", "b_16_16", "c_16_16", "d_16_16", "e", "f"},
        label,
    )
    result = []
    for field in ("a_16_16", "b_16_16", "c_16_16", "d_16_16", "e", "f"):
        member = matrix[field]
        if isinstance(member, bool) or not isinstance(member, int):
            raise PrecomposedVectorError(f"{label}.{field}: expected integer")
        result.append(member)
    return result


def _contained_regular_file(root: Path, relative: str, label: str) -> Path:
    path = Path(relative)
    if path.is_absolute() or ".." in path.parts or "\\" in relative:
        raise PrecomposedVectorError(f"{label}: unsafe relative path")
    candidate = root
    for component in path.parts:
        candidate = candidate / component
        if candidate.is_symlink():
            raise PrecomposedVectorError(f"{label}: symlink path component is forbidden")
    if not candidate.is_file():
        raise PrecomposedVectorError(f"{label}: expected contained regular file")
    return candidate


def _canonical_utf8_text_file(root: Path, relative: str, label: str) -> bytes:
    payload = _contained_regular_file(root, relative, label).read_bytes()
    if (
        not payload.endswith(b"\n")
        or b"\r" in payload
        or b"\0" in payload
        or payload.startswith(b"\xef\xbb\xbf")
    ):
        raise PrecomposedVectorError(
            f"{label}: text must be UTF-8, LF, final-LF, without BOM or NUL"
        )
    try:
        text = payload.decode("utf-8")
    except UnicodeError as error:
        raise PrecomposedVectorError(f"{label}: text is not valid UTF-8") from error
    if not text[:-1]:
        raise PrecomposedVectorError(f"{label}: text is empty")
    return payload


def _artifact_set_digest(artifacts: Iterable[tuple[str, bytes]]) -> str:
    digest = hashlib.sha256()
    for name, payload in sorted(artifacts):
        encoded = name.encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    return digest.hexdigest()


def _atomic_json_write(path: Path, value: Any) -> None:
    payload = canonical_json_bytes(value) + b"\n"
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    if temporary.exists():
        raise PrecomposedVectorError(f"temporary output already exists: {temporary}")
    try:
        with temporary.open("xb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def _publication_root(repository: Path) -> Path:
    return repository / "samples/machine-package/staging/production-book-1"


def _production_component_records() -> list[dict[str, Any]]:
    return [
        {
            "id": PRODUCTION_COMPONENTS[0],
            "media": ["png"],
            "proofs": ["accessibility-manifest.json", "accessibility.pdf"],
        },
        {
            "id": PRODUCTION_COMPONENTS[1],
            "media": ["svg-safe-1", "svg-safe-2"],
            "proofs": [
                "safe-vector-1-manifest.json",
                "safe-vector-1.pdf",
                "safe-vector-2-manifest.json",
                "safe-vector-2.pdf",
            ],
        },
        {
            "id": PRODUCTION_COMPONENTS[2],
            "media": ["jpeg-baseline"],
            "proofs": ["jpeg-manifest.json", "jpeg.pdf"],
        },
        {
            "id": PRODUCTION_COMPONENTS[3],
            "media": ["sfnt-truetype-glyf", "ttc-truetype-glyf"],
            "proofs": ["math-manifest.json", "math.pdf"],
        },
        {
            "id": PRODUCTION_COMPONENTS[4],
            "media": ["sfnt-cff1"],
            "proofs": ["cff-manifest.json", "cff.pdf"],
        },
    ]


def _production_resource_records(repository: Path) -> list[dict[str, Any]]:
    root = _publication_root(repository)
    expectation = _load_json(root / "publication-expectation.json")
    records = expectation.get("resource_hashes") if isinstance(expectation, dict) else None
    if not isinstance(records, list) or len(records) != 73:
        raise PrecomposedVectorError("publication expectation does not contain 73 resources")
    expected_uris: set[str] = set()
    for pattern in PRODUCTION_RESOURCE_PATTERNS:
        for path in root.glob(pattern):
            if path.is_symlink() or not path.is_file():
                raise PrecomposedVectorError(
                    f"publication input is not a regular file: {path.relative_to(root)}"
                )
            expected_uris.add(path.relative_to(root).as_posix())
    actual_uris = [row.get("uri") if isinstance(row, dict) else None for row in records]
    if actual_uris != sorted(expected_uris) or len(expected_uris) != 73:
        raise PrecomposedVectorError(
            "publication resource ledger is not the exact production input set"
        )
    normalized: list[dict[str, Any]] = []
    previous = ""
    for index, raw in enumerate(records):
        row = _exact_object(raw, {"bytes", "sha256", "uri"}, f"publication resource {index}")
        uri = row.get("uri")
        if not isinstance(uri, str) or uri <= previous:
            raise PrecomposedVectorError("publication resources are duplicated or noncanonical")
        previous = uri
        path = _contained_regular_file(root, uri, f"publication resource {index}")
        payload = path.read_bytes()
        normalized_row = {
            "bytes": _integer(row.get("bytes"), f"publication resource {index}.bytes"),
            "sha256": _hash(row.get("sha256"), f"publication resource {index}.sha256"),
            "uri": uri,
        }
        if normalized_row["bytes"] != len(payload) or normalized_row["sha256"] != _sha256(payload):
            raise PrecomposedVectorError(f"publication resource differs: {uri}")
        normalized.append(normalized_row)
    return normalized


def _verify_production_proof_pairs(
    manifests: dict[str, Any], pdf_payloads: dict[str, bytes]
) -> None:
    for manifest_name, algorithm in PRODUCTION_MANIFEST_ALGORITHMS.items():
        manifest = manifests.get(manifest_name)
        if not isinstance(manifest, dict) or manifest.get("algorithm") != algorithm:
            raise PrecomposedVectorError(
                f"production proof has the wrong manifest identity: {manifest_name}"
            )
        pdf_name = manifest_name.removesuffix("-manifest.json") + ".pdf"
        pdf_payload = pdf_payloads[pdf_name]
        if manifest_name in PRODUCTION_DIRECT_PDF_HASH_MANIFESTS:
            if manifest.get("pdf_sha256") != _sha256(pdf_payload):
                raise PrecomposedVectorError(
                    f"production proof manifest targets a different PDF: {manifest_name}"
                )
        elif manifest_name in {
            "accessibility-manifest.json",
            "navigation-manifest.json",
        }:
            pdf = manifest.get("pdf")
            fingerprints = manifest.get("fingerprints")
            if (
                manifest.get("contract") != "typaxis.contract/1.4"
                or manifest.get("profile_id") != PRODUCTION_PROFILE
                or not isinstance(pdf, dict)
                or pdf.get("byte_length") != len(pdf_payload)
                or not isinstance(fingerprints, dict)
                or fingerprints.get("pdf_sha256") != _sha256(pdf_payload)
            ):
                raise PrecomposedVectorError(
                    f"production proof manifest has stale profile/PDF facts: {manifest_name}"
                )
    if manifests.get("cff-manifest.json", {}).get("resource_profile_id") != (
        "typaxis.resource-profile/sfnt-cff1/1"
    ):
        raise PrecomposedVectorError("CFF proof has the wrong resource profile")
    vector_manifest = manifests.get("safe-vector-2-manifest.json")
    if not isinstance(vector_manifest, dict) or vector_manifest.get("status") != "built":
        raise PrecomposedVectorError("SafeVector /2 proof is not a built manifest")


def _build_production_readiness_receipt(
    directory: Path, repository: Path
) -> dict[str, Any]:
    entries = list(directory.iterdir())
    invalid_entries = sorted(
        entry.name
        for entry in entries
        if entry.is_symlink() or not entry.is_file()
    )
    if invalid_entries:
        raise PrecomposedVectorError(
            f"production proof directory contains invalid entries: {invalid_entries}"
        )
    actual = {
        entry.name
        for entry in entries
    }
    if actual not in (PRODUCTION_PROOF_FILES, PRODUCTION_PROOF_FILES | {"production-resource-set-receipt.json"}):
        raise PrecomposedVectorError(
            "production proof directory has missing or extra files: "
            f"{sorted(actual ^ PRODUCTION_PROOF_FILES)}"
        )
    artifacts = []
    manifests: dict[str, Any] = {}
    pdf_payloads: dict[str, bytes] = {}
    for name in sorted(PRODUCTION_PROOF_FILES):
        path = directory / name
        if path.is_symlink() or not path.is_file():
            raise PrecomposedVectorError(f"production proof is not a regular file: {name}")
        payload = path.read_bytes()
        if not payload:
            raise PrecomposedVectorError(f"production proof is empty: {name}")
        if name.endswith(".json"):
            manifests[name] = _load_json(path)
        elif not payload.startswith(b"%PDF-"):
            raise PrecomposedVectorError(f"production proof is not a PDF: {name}")
        else:
            pdf_payloads[name] = payload
        artifacts.append({"bytes": len(payload), "name": name, "sha256": _sha256(payload)})
    _verify_production_proof_pairs(manifests, pdf_payloads)
    root = _publication_root(repository)
    capability_path = root / "publication-capabilities.json"
    expectation_path = root / "publication-expectation.json"
    resources = _production_resource_records(repository)
    return {
        "artifacts": artifacts,
        "capabilities_sha256": _sha256(capability_path.read_bytes()),
        "components": _production_component_records(),
        "contract": PRODUCTION_READINESS_CONTRACT,
        "expectation_sha256": _sha256(expectation_path.read_bytes()),
        "font_media": PRODUCTION_FONT_MEDIA,
        "image_media": PRODUCTION_IMAGE_MEDIA,
        "integration_proofs": [
            "accessibility-manifest.json",
            "accessibility.pdf",
            "math-manifest.json",
            "math.pdf",
            "navigation-manifest.json",
            "navigation.pdf",
            "semantic-manifest.json",
            "semantic.pdf",
        ],
        "profile": PRODUCTION_PROFILE,
        "resource_count": len(resources),
        "resource_ledger_sha256": _sha256(canonical_json_bytes(resources)),
    }


def write_production_readiness_receipt(directory: Path, repository: Path) -> Path:
    directory = directory.resolve(strict=True)
    if not directory.is_dir():
        raise PrecomposedVectorError("production proof path is not a directory")
    receipt = _build_production_readiness_receipt(directory, repository)
    output = directory / "production-resource-set-receipt.json"
    _atomic_json_write(output, receipt)
    return output


def verify_production_readiness(
    directory: Path,
    repository: Path,
    *,
    vector_pdf: bytes,
    vector_manifest: bytes,
) -> dict[str, Any]:
    directory = directory.resolve(strict=True)
    receipt_path = directory / "production-resource-set-receipt.json"
    if receipt_path.is_symlink() or not receipt_path.is_file():
        raise PrecomposedVectorError("production resource-set receipt is not a regular file")
    receipt = _load_json(receipt_path)
    if receipt_path.read_bytes() != canonical_json_bytes(receipt) + b"\n":
        raise PrecomposedVectorError("production resource-set receipt is not JCS plus LF")
    expected = _build_production_readiness_receipt(directory, repository)
    if receipt != expected:
        raise PrecomposedVectorError("production resource-set receipt differs from its proof set")
    if (directory / "safe-vector-2.pdf").read_bytes() != vector_pdf or (
        directory / "safe-vector-2-manifest.json"
    ).read_bytes() != vector_manifest:
        raise PrecomposedVectorError("SafeVector /2 production proof differs from the V18 output")
    resources = _production_resource_records(repository)
    font_uris = {
        "accessibility/job/body.ttf",
        "accessibility/job/collection.ttc",
        "cff-media/typaxis-cff-fixture.otf.hex",
        "math/job/math.ttf",
    }
    fonts = [row for row in resources if row["uri"] in font_uris]
    if {row["uri"] for row in fonts} != font_uris:
        raise PrecomposedVectorError("production font identity set is incomplete")
    return {
        "capabilities_sha256": expected["capabilities_sha256"],
        "expectation_sha256": expected["expectation_sha256"],
        "font_resources": fonts,
        "readiness_sha256": _sha256(receipt_path.read_bytes()),
        "resource_count": len(resources),
        "resource_ledger_sha256": expected["resource_ledger_sha256"],
        "resources": resources,
    }


def _resolve_external_tool(name: str, override: str | None) -> Path:
    candidate = override
    if candidate is None:
        candidate = shutil.which(name)
    elif os.sep not in candidate and (os.altsep is None or os.altsep not in candidate):
        candidate = shutil.which(candidate)
    if candidate is None:
        raise PrecomposedVectorError(f"required external tool is unavailable: {name}")
    path = Path(candidate).resolve(strict=True)
    if not path.is_file():
        raise PrecomposedVectorError(f"external tool is not a regular file: {path}")
    return path


def _capture(command: list[str]) -> tuple[bytes, bytes]:
    try:
        completed = subprocess.run(
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=EXTERNAL_COMMAND_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        raise PrecomposedVectorError(
            f"external command exceeded {EXTERNAL_COMMAND_TIMEOUT_SECONDS} seconds: {command[0]}"
        ) from error
    except OSError as error:
        raise PrecomposedVectorError(f"cannot execute {command[0]}: {error}") from error
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise PrecomposedVectorError(
            f"external command failed ({completed.returncode}): {command[0]}: {detail}"
        )
    return completed.stdout, completed.stderr


def _first_version_line(path: Path, arguments: list[str]) -> str:
    stdout, stderr = _capture([os.fspath(path), *arguments])
    lines = (stdout + stderr).decode("utf-8", "replace").splitlines()
    try:
        return next(line.strip() for line in lines if line.strip())
    except StopIteration as error:
        raise PrecomposedVectorError(f"external tool returned no version: {path}") from error


def _tree_payload_sha256(root: Path) -> str:
    payloads: list[tuple[str, bytes]] = []
    for path in sorted(root.rglob("*"), key=lambda item: os.fsencode(item.relative_to(root))):
        if path.is_symlink():
            raise PrecomposedVectorError(f"tool payload contains a symlink: {path}")
        if path.is_file():
            payloads.append((path.relative_to(root).as_posix(), path.read_bytes()))
        elif not path.is_dir():
            raise PrecomposedVectorError(f"tool payload contains a special file: {path}")
    if not payloads:
        raise PrecomposedVectorError(f"tool payload is empty: {root}")
    return _artifact_set_digest(payloads)


def _tool_record(
    name: str,
    path: Path,
    version_arguments: list[str],
    *,
    payload_root: Path | None = None,
) -> dict[str, str]:
    executable_sha256 = _sha256(path.read_bytes())
    return {
        "executable_sha256": executable_sha256,
        "name": name,
        "payload_sha256": (
            _tree_payload_sha256(payload_root) if payload_root is not None else executable_sha256
        ),
        "version": _first_version_line(path, version_arguments),
    }


def _verify_verapdf(path: Path, pdf: Path) -> dict[str, Any]:
    stdout, stderr = _capture(
        [os.fspath(path), "-f", "ua1", "--format", "xml", os.fspath(pdf)]
    )
    if stderr:
        raise PrecomposedVectorError(
            "veraPDF emitted stderr instead of a clean validation result: "
            + stderr.decode("utf-8", "replace").strip()
        )
    try:
        root = ET.fromstring(stdout)
    except ET.ParseError as error:
        raise PrecomposedVectorError(f"veraPDF returned malformed XML: {error}") from error
    releases = {
        element.attrib.get("id"): element.attrib.get("version")
        for element in root.findall("./buildInformation/releaseDetails")
    }
    if releases != {"apps": "1.30.2", "core": "1.30.2", "validation-model": "1.30.2"}:
        raise PrecomposedVectorError(f"veraPDF release components are not pinned 1.30.2: {releases}")
    jobs = root.findall("./jobs/job")
    if len(jobs) != 1:
        raise PrecomposedVectorError("veraPDF did not report exactly one job")
    report = jobs[0].find("validationReport")
    details = report.find("details") if report is not None else None
    if (
        report is None
        or details is None
        or report.attrib.get("jobEndStatus") != "normal"
        or report.attrib.get("profileName") != "PDF/UA-1 validation profile"
        or report.attrib.get("isCompliant") != "true"
        or details.attrib
        != {
            "failedChecks": "0",
            "failedRules": "0",
            "passedChecks": "172",
            "passedRules": "106",
        }
    ):
        raise PrecomposedVectorError("veraPDF PDF/UA-1 report is warning, failed, or unpinned")
    batch = root.find("./batchSummary")
    validation_reports = root.find("./batchSummary/validationReports")
    if (
        batch is None
        or validation_reports is None
        or any(
            batch.attrib.get(name) != "0"
            for name in ("encrypted", "failedToParse", "outOfMemory", "veraExceptions")
        )
        or batch.attrib.get("totalJobs") != "1"
        or validation_reports.attrib
        != {"compliant": "1", "failedJobs": "0", "nonCompliant": "0"}
        or validation_reports.text != "1"
    ):
        raise PrecomposedVectorError("veraPDF batch summary contains an error or warning result")
    return {
        "failed_checks": 0,
        "failed_rules": 0,
        "flavour": "ua1",
        "passed_checks": 172,
        "passed_rules": 106,
        "profile": "PDF/UA-1 validation profile",
        "version": "1.30.2",
    }


def verify_external_evidence(
    *,
    artifact_directory: Path,
    repository: Path,
    mutool: str | None,
    pdftotext: str | None,
    pdfinfo: str | None,
    verapdf: str | None,
    binary: str | None,
) -> tuple[dict[str, Any], dict[str, str], list[dict[str, str]]]:
    pdf = artifact_directory / "output.pdf"
    pdf_payload = pdf.read_bytes()
    pdf_sha256 = _sha256(pdf_payload)
    paths = {
        "mutool": _resolve_external_tool("mutool", mutool),
        "pdfinfo": _resolve_external_tool("pdfinfo", pdfinfo),
        "pdftotext": _resolve_external_tool("pdftotext", pdftotext),
        "verapdf": _resolve_external_tool("verapdf", verapdf),
    }
    policy_path = _publication_root(repository) / "external-tool-policy.json"
    policy = _load_json(policy_path)
    expected_policy = {
        "contract": "typaxis.external-pdf-tool-policy/1",
        "mupdf_source": {
            "sha256": "44075a84e329db55b9bef5f342a70fd26d69e48ad1d33cb89d9664581c641156",
            "url": "https://mupdf.com/downloads/archive/mupdf-1.28.2-source.tar.gz",
        },
        "mutool_version": "mutool version 1.28.2",
        "poppler_source": {
            "sha256": "dc906e68cea698109706ac6aa3d2c9d4512fcfcac42d90b8afcda486d1b9abd0",
            "url": "https://poppler.freedesktop.org/poppler-26.08.0.tar.xz",
        },
        "poppler_version": "26.08.0",
        "render_dpis": [72, 144, 288],
        "verapdf": {
            "flavour": "ua1",
            "installer_sha256": "6cc6341cb1af644044054b81f00a6590a7918abb18f762243de115258bcad838",
            "payload_sha256": "e12acf5b4dd4d03b4e3abaf88ddb0ecccfc914afe65299f50681853d6ce5b63b",
            "signature_sha256": "f33175e402f28c42e80866aa62aa337c5d7d7a16a4ea1ae4ff50b0f13343ff26",
            "signer_fingerprint": "13DD102B4DD69354D12DE5A83184863278B17FE7",
            "version": "1.30.2",
        },
    }
    if policy != expected_policy:
        raise PrecomposedVectorError("external PDF tool policy is not the pinned V19 policy")
    binary_path = _resolve_external_tool("typaxis", binary)
    try:
        differential = pdf_differential.verify_pdf_differential(
            [pdf],
            expected_text="(1)",
            expected_pages=2,
            mutool=os.fspath(paths["mutool"]),
            pdftotext=os.fspath(paths["pdftotext"]),
            pdfinfo=os.fspath(paths["pdfinfo"]),
            render_dpis=(72, 144, 288),
            vector_expectations=pdf_differential.VectorPdfExpectations(1, 1, 4, 2),
        )
        structure = pdf_differential.inspect_vector_pdf_structure(
            pdf_payload, pdf_differential.VectorPdfExpectations(1, 1, 4, 2)
        )
    except (OSError, pdf_differential.PdfDifferentialError) as error:
        raise PrecomposedVectorError(f"external PDF differential failed: {error}") from error
    vera = _verify_verapdf(paths["verapdf"], pdf)

    publication = _publication_root(repository)
    assessment_path = publication / "matterhorn-assessment.json"
    assessment = _load_json(assessment_path)
    expected_assessment = matterhorn_protocol.build_assessment(
        pdf_sha256=pdf_sha256,
        fixture_revision_sha256=_sha256(
            (publication / "publication-expectation.json").read_bytes()
        ),
    )
    if assessment != expected_assessment:
        raise PrecomposedVectorError("Matterhorn /2 assessment is stale or incomplete")
    method_counts = {
        method: sum(item["method"] == method for item in assessment["items"])
        for method in ("human", "machine", "no_specific_test")
    }
    status_counts = {
        status: sum(item["status"] == status for item in assessment["items"])
        for status in ("not_applicable", "passed")
    }
    if method_counts != {"human": 47, "machine": 87, "no_specific_test": 2} or status_counts != {
        "not_applicable": 37,
        "passed": 99,
    }:
        raise PrecomposedVectorError("Matterhorn /2 method or status closure differs")

    tools = [
        _tool_record("mutool", paths["mutool"], ["-v"]),
        _tool_record("pdfinfo", paths["pdfinfo"], ["-v"]),
        _tool_record("pdftotext", paths["pdftotext"], ["-v"]),
        _tool_record(
            "verapdf",
            paths["verapdf"],
            ["--version"],
            payload_root=paths["verapdf"].parent,
        ),
    ]
    tools.sort(key=lambda record: record["name"])
    expected_versions = {
        "mutool": policy["mutool_version"],
        "pdfinfo": f"pdfinfo version {policy['poppler_version']}",
        "pdftotext": f"pdftotext version {policy['poppler_version']}",
        "verapdf": f"veraPDF {policy['verapdf']['version']}",
    }
    if {tool["name"]: tool["version"] for tool in tools} != expected_versions:
        raise PrecomposedVectorError("external PDF tool versions differ from the pinned policy")
    if next(tool for tool in tools if tool["name"] == "verapdf")[
        "payload_sha256"
    ] != policy["verapdf"]["payload_sha256"]:
        raise PrecomposedVectorError("veraPDF payload differs from the pinned policy")
    binary_record = {
        "sha256": _sha256(binary_path.read_bytes()),
        "version": _first_version_line(binary_path, ["--version"]),
    }
    if binary_record["version"] != "typaxis 0.1.0":
        raise PrecomposedVectorError("Typaxis binary version differs from the source release")
    external = {
        "differential": {
            "extracted_text_sha256": differential.extracted_text_sha256,
            "page_count": differential.page_count,
            "render_dpis": list(differential.render_dpis),
            "render_sha256": differential.render_sha256,
        },
        "matterhorn": {
            "assessment_sha256": _sha256(assessment_path.read_bytes()),
            "human_item_count": method_counts["human"],
            "item_count": len(assessment["items"]),
            "machine_item_count": method_counts["machine"],
            "not_applicable_count": status_counts["not_applicable"],
            "passed_count": status_counts["passed"],
        },
        "pdf_sha256": pdf_sha256,
        "tool_policy_sha256": _sha256(policy_path.read_bytes()),
        "structure": {
            "do_count": structure.do_count,
            "ext_g_state_count": structure.ext_g_state_count,
            "form_count": structure.form_count,
            "page_root_y_flip_count": structure.page_root_y_flip_count,
            "structure_sha256": structure.structure_sha256,
        },
        "verapdf": vera,
    }
    return external, binary_record, tools


def _artifact_index(directory: Path) -> tuple[list[dict[str, Any]], dict[str, bytes]]:
    index_path = directory / "artifact-index.json"
    if index_path.is_symlink() or not index_path.is_file():
        raise PrecomposedVectorError("artifact index is not a regular file")
    index = _exact_object(
        _load_json(index_path),
        {"artifacts", "contract"},
        "artifact index",
    )
    if index["contract"] != ARTIFACT_CONTRACT or not isinstance(index["artifacts"], list):
        raise PrecomposedVectorError("artifact index contract or artifacts array is invalid")
    records: list[dict[str, Any]] = []
    payloads: dict[str, bytes] = {}
    previous = ""
    for offset, raw in enumerate(index["artifacts"]):
        record = _exact_object(raw, {"bytes", "name", "sha256"}, f"artifact[{offset}]")
        name = record["name"]
        if not isinstance(name, str) or name not in EXPECTED_ARTIFACTS or name <= previous:
            raise PrecomposedVectorError("artifact names are unknown, duplicated, or noncanonical")
        previous = name
        path = directory / name
        if path.is_symlink() or not path.is_file():
            raise PrecomposedVectorError(f"artifact is not a contained regular file: {name}")
        payload = path.read_bytes()
        if _integer(record["bytes"], f"{name}.bytes") != len(payload):
            raise PrecomposedVectorError(f"artifact byte length differs: {name}")
        if _hash(record["sha256"], f"{name}.sha256") != _sha256(payload):
            raise PrecomposedVectorError(f"artifact hash differs: {name}")
        records.append(record)
        payloads[name] = payload
    if set(payloads) != EXPECTED_ARTIFACTS:
        raise PrecomposedVectorError("artifact index does not contain the exact required set")
    allowed = EXPECTED_ARTIFACTS | {"artifact-index.json"}
    non_files = [
        entry.name
        for entry in directory.iterdir()
        if not (entry.is_file() or entry.is_symlink())
    ]
    if non_files:
        raise PrecomposedVectorError(
            f"artifact directory contains non-file entries: {sorted(non_files)}"
        )
    actual = {
        entry.name
        for entry in directory.iterdir()
        if entry.is_file() or entry.is_symlink()
    }
    if actual != allowed:
        raise PrecomposedVectorError(
            f"artifact directory has missing or unindexed files: {sorted(actual ^ allowed)}"
        )
    return records, payloads


def _decode_pdf_utf16_hex(value: bytes, label: str) -> str:
    try:
        raw = bytes.fromhex(value.decode("ascii"))
    except (UnicodeError, ValueError) as error:
        raise PrecomposedVectorError(f"{label} is not hexadecimal") from error
    if not raw.startswith(b"\xfe\xff"):
        raise PrecomposedVectorError(f"{label} is not UTF-16BE with BOM")
    try:
        return raw[2:].decode("utf-16-be")
    except UnicodeError as error:
        raise PrecomposedVectorError(f"{label} is not valid UTF-16BE") from error


def _decode_actual_text(pdf: bytes) -> list[str]:
    return [
        _decode_pdf_utf16_hex(match.group(1), "PDF ActualText")
        for match in _ACTUAL_TEXT.finditer(pdf)
    ]


def _decode_marked_content(pdf: bytes) -> list[tuple[str, str, str]]:
    decoded = [
        (
            match.group(1).decode("ascii"),
            _decode_pdf_utf16_hex(match.group(2), "PDF ActualText"),
            _decode_pdf_utf16_hex(match.group(3), "PDF Lang"),
        )
        for match in _MARKED_CONTENT.finditer(pdf)
    ]
    if len(decoded) != len(_ACTUAL_TEXT.findall(pdf)):
        raise PrecomposedVectorError(
            "PDF contains ActualText outside the supported semantic marked-content shape"
        )
    return decoded


def _verify_classic_xref(
    pdf: bytes, label: str, objects: list[tuple[int, bytes]]
) -> None:
    match = _STARTXREF.search(pdf)
    if match is None:
        raise PrecomposedVectorError(f"{label} has no canonical startxref")
    xref_offset = int(match.group(1))
    header = f"xref\n0 {len(objects) + 1}\n".encode("ascii")
    if xref_offset >= len(pdf) or not pdf[xref_offset:].startswith(header):
        raise PrecomposedVectorError(f"{label} startxref or xref size differs")
    cursor = xref_offset + len(header)
    entries: list[bytes] = []
    for _ in range(len(objects) + 1):
        end = pdf.find(b"\n", cursor)
        if end < 0:
            raise PrecomposedVectorError(f"{label} xref entry is truncated")
        entries.append(pdf[cursor : end + 1])
        cursor = end + 1
    if entries[0] != b"0000000000 65535 f \n":
        raise PrecomposedVectorError(f"{label} xref free entry differs")
    for (number, _), entry in zip(objects, entries[1:], strict=True):
        expected_offset = pdf.find(f"{number} 0 obj\n".encode("ascii"))
        expected = f"{expected_offset:010d} 00000 n \n".encode("ascii")
        if expected_offset < 0 or entry != expected:
            raise PrecomposedVectorError(
                f"{label} xref offset differs for object {number}"
            )
    trailer = pdf[cursor : match.start()]
    if not trailer.startswith(b"trailer\n") or (
        f"/Size {len(objects) + 1} ".encode("ascii") not in trailer
    ):
        raise PrecomposedVectorError(f"{label} trailer size differs")


def _verify_vector_pdf_shape(
    pdf: bytes, label: str, expected_forms: int, expected_do: int
) -> list[tuple[int, bytes]]:
    if not pdf.startswith(b"%PDF-1.7\n") or not pdf.endswith(b"%%EOF\n"):
        raise PrecomposedVectorError(f"{label} is not the deterministic PDF 1.7 subset")
    if b"xref\n" not in pdf or b"trailer\n" not in pdf:
        raise PrecomposedVectorError(f"{label} lacks classic xref/trailer closure")
    objects = [(int(number), body) for number, body in _OBJECT.findall(pdf)]
    if not objects or [number for number, _ in objects] != list(range(1, len(objects) + 1)):
        raise PrecomposedVectorError(f"{label} indirect objects are not dense")
    _verify_classic_xref(pdf, label, objects)
    if _RASTER_IMAGE.search(pdf):
        raise PrecomposedVectorError(f"{label} contains raster image content")
    forms = [body for _, body in objects if b"/Subtype /Form" in body]
    if len(forms) != expected_forms:
        raise PrecomposedVectorError(f"{label} Form count differs")
    for form in forms:
        if (
            not all(token in form for token in (b"/BBox [", b"stream\n", b" m\n"))
            or not any(token in form for token in (b" l\n", b" c\n", b" re "))
        ):
            raise PrecomposedVectorError(f"{label} vector Form lost BBox or path operators")
        if any(token in form for token in (b"/MCID", b"/Alt", b"/ActualText", b"/Lang", b" BDC")):
            raise PrecomposedVectorError(f"{label} reusable Form contains semantic marked content")
    if len(_DO.findall(pdf)) != expected_do:
        raise PrecomposedVectorError(f"{label} Do count differs")
    return objects


def _verify_pdf(pdf: bytes, counts: dict[str, Any], corpus: Path) -> list[str]:
    expected_forms = _integer(counts["forms"], "counts.forms", 1)
    expected_do = _integer(counts["do"], "counts.do", 1)
    objects = _verify_vector_pdf_shape(pdf, "output.pdf", expected_forms, expected_do)
    expected_objects = _integer(counts["objects"], "counts.objects", 1)
    if len(objects) != expected_objects:
        raise PrecomposedVectorError("PDF object count differs from verification receipt")
    if b"/S /Formula" not in pdf or b"/S /Figure" not in pdf or b"/Alt <FEFF" not in pdf:
        raise PrecomposedVectorError("PDF lacks Formula/Figure/Alt structure semantics")
    for source in sorted((corpus / "tex").glob("*.tex")):
        tex = source.read_bytes().strip()
        if tex and tex in pdf:
            raise PrecomposedVectorError(f"opaque source TeX leaked into PDF: {source.name}")
    return _decode_actual_text(pdf)


def _verify_build_manifest_root(
    root: Any, expected_pdf_sha256: str, label: str
) -> dict[str, dict[str, Any]]:
    prefixes = ("book_navigation", "math_vector", "safe_vector", "tagged_pdf")
    expected_keys = {"status"}
    for prefix in prefixes:
        expected_keys.update({f"{prefix}_manifest", f"{prefix}_manifest_fingerprint"})
    value = _exact_object(root, expected_keys, label)
    if value["status"] != "built":
        raise PrecomposedVectorError(f"{label} is not built")
    children: dict[str, dict[str, Any]] = {}
    for prefix in prefixes:
        child = value[f"{prefix}_manifest"]
        if not isinstance(child, dict):
            raise PrecomposedVectorError(f"{label}.{prefix}_manifest is not an object")
        fingerprint = _sha256(canonical_json_bytes(child))
        if value[f"{prefix}_manifest_fingerprint"] != fingerprint:
            raise PrecomposedVectorError(f"{label}.{prefix} fingerprint differs")
        children[prefix] = child
    for prefix in ("book_navigation", "safe_vector", "tagged_pdf"):
        fingerprints = children[prefix].get("fingerprints")
        if (
            not isinstance(fingerprints, dict)
            or fingerprints.get("pdf_sha256") != expected_pdf_sha256
        ):
            raise PrecomposedVectorError(f"{label}.{prefix} does not close the final PDF")
    safe_fingerprint = value["safe_vector_manifest_fingerprint"]
    math_fingerprints = children["math_vector"].get("fingerprints")
    tagged_fingerprints = children["tagged_pdf"].get("fingerprints")
    if (
        not isinstance(math_fingerprints, dict)
        or math_fingerprints.get("safe_vector_manifest_sha256") != safe_fingerprint
        or not isinstance(tagged_fingerprints, dict)
        or tagged_fingerprints.get("safe_vector_manifest_sha256") != safe_fingerprint
        or tagged_fingerprints.get("math_vector_manifest_sha256")
        != value["math_vector_manifest_fingerprint"]
    ):
        raise PrecomposedVectorError(f"{label} child dependency fingerprints differ")
    return children


def _verify_corpus_admission(admission: Any, resources: list[dict[str, str]]) -> None:
    receipt = _exact_object(
        admission,
        {
            "algorithm",
            "alias_count",
            "candidate_count",
            "candidates",
            "relative_object_role_count_if_all_candidates_selected",
        },
        "corpus admission",
    )
    candidates = receipt["candidates"]
    if (
        receipt["algorithm"] != "typaxis.vector-form-dedupe/1"
        or receipt["alias_count"] != 13
        or receipt["candidate_count"] != 12
        or not isinstance(candidates, list)
        or len(candidates) != 12
    ):
        raise PrecomposedVectorError("corpus admission count or algorithm differs")
    aliases_by_id: dict[int, dict[str, Any]] = {}
    candidate_keys: list[tuple[str, str, str, str, str]] = []
    alias_groups: list[list[int]] = []
    limits_fingerprints: set[str] = set()
    profile_fingerprints: set[str] = set()
    relative_roles = 0
    for offset, raw_candidate in enumerate(candidates):
        candidate = _exact_object(
            raw_candidate,
            {
                "aliases",
                "ext_g_state_plan",
                "intrinsic_height",
                "intrinsic_width",
                "key",
                "relative_object_role_count_if_selected",
                "view_box",
            },
            f"corpus candidate[{offset}]",
        )
        key = _exact_object(
            candidate["key"],
            {"ir_fingerprint", "ir_id", "media_type", "parser_id", "source_sha256"},
            f"corpus candidate[{offset}].key",
        )
        if (
            key["media_type"] != "svg-safe-2"
            or key["parser_id"] != "typaxis.safe-svg-parser/2"
            or key["ir_id"] != "typaxis.safe-vector-ir/2"
        ):
            raise PrecomposedVectorError("corpus candidate uses the wrong vector profile")
        source_hash = _hash(key["source_sha256"], "corpus candidate source hash")
        ir_hash = _hash(key["ir_fingerprint"], "corpus candidate IR hash")
        candidate_keys.append(
            (source_hash, key["media_type"], key["parser_id"], key["ir_id"], ir_hash)
        )
        if (
            _integer(candidate["intrinsic_width"], "corpus intrinsic width", 1) <= 0
            or _integer(candidate["intrinsic_height"], "corpus intrinsic height", 1) <= 0
            or not isinstance(candidate["view_box"], list)
            or len(candidate["view_box"]) != 4
            or any(
                isinstance(value, bool) or not isinstance(value, int)
                for value in candidate["view_box"]
            )
            or candidate["view_box"][2] <= 0
            or candidate["view_box"][3] <= 0
        ):
            raise PrecomposedVectorError("corpus candidate intrinsic geometry differs")
        roles = _integer(
            candidate["relative_object_role_count_if_selected"],
            "corpus candidate relative roles",
            2,
        )
        relative_roles += roles
        ext_states = candidate["ext_g_state_plan"]
        if not isinstance(ext_states, list) or len(ext_states) != roles - 1:
            raise PrecomposedVectorError("corpus candidate ExtGState plan differs")
        observed_roles: list[int] = []
        for state_offset, raw_state in enumerate(ext_states):
            state = _exact_object(
                raw_state,
                {"fill_alpha", "relative_object_role", "stroke_alpha"},
                f"corpus candidate[{offset}].ExtGState[{state_offset}]",
            )
            fill_alpha = _integer(state["fill_alpha"], "corpus fill alpha")
            stroke_alpha = _integer(state["stroke_alpha"], "corpus stroke alpha")
            if fill_alpha > 65_536 or stroke_alpha > 65_536 or not (fill_alpha or stroke_alpha):
                raise PrecomposedVectorError("corpus candidate alpha differs")
            observed_roles.append(
                _integer(state["relative_object_role"], "corpus ExtGState role", 1)
            )
        if observed_roles != list(range(1, roles)):
            raise PrecomposedVectorError("corpus candidate object roles are not dense")
        aliases = candidate["aliases"]
        if not isinstance(aliases, list) or not aliases:
            raise PrecomposedVectorError("corpus candidate has no aliases")
        group: list[int] = []
        for raw_alias in aliases:
            alias = _exact_object(
                raw_alias,
                {
                    "admission_allocation_charge",
                    "admitted_sha256",
                    "expected_sha256",
                    "image_id",
                    "limits_fingerprint",
                    "profile_fingerprint",
                    "provenance",
                    "uri",
                },
                "corpus alias",
            )
            image_id = _integer(alias["image_id"], "corpus alias image_id")
            if image_id in aliases_by_id:
                raise PrecomposedVectorError("corpus admission duplicates an image ID")
            if _hash(alias["admitted_sha256"], "corpus admitted hash") != source_hash:
                raise PrecomposedVectorError("corpus alias and content-key hashes differ")
            _integer(
                alias["admission_allocation_charge"],
                "corpus admission allocation charge",
                1,
            )
            limits_fingerprints.add(
                _hash(alias["limits_fingerprint"], "corpus limits fingerprint")
            )
            profile_fingerprints.add(
                _hash(alias["profile_fingerprint"], "corpus profile fingerprint")
            )
            aliases_by_id[image_id] = alias
            group.append(image_id)
        if group != sorted(group):
            raise PrecomposedVectorError("corpus aliases are not in numeric ID order")
        alias_groups.append(group)
    if candidate_keys != sorted(candidate_keys):
        raise PrecomposedVectorError("corpus content keys are not in canonical order")
    if receipt["relative_object_role_count_if_all_candidates_selected"] != relative_roles:
        raise PrecomposedVectorError("corpus aggregate object-role count differs")
    expected_alias_groups = [[0, 1], *[[value] for value in range(2, 13)]]
    if (
        set(aliases_by_id) != set(range(13))
        or sorted(alias_groups) != expected_alias_groups
        or len(limits_fingerprints) != 1
        or len(profile_fingerprints) != 1
    ):
        raise PrecomposedVectorError("corpus alias grouping differs")
    for row in resources:
        image_id = _tsv_integer(row["image_id"], "resource image_id", minimum=0)
        alias = aliases_by_id[image_id]
        provenance = _exact_object(
            alias["provenance"],
            {"engine_id", "engine_version", "rules_version"},
            f"corpus alias {image_id} provenance",
        )
        if (
            alias["uri"] != row["svg_path"]
            or alias["expected_sha256"] != row["expected_sha256"]
            or alias["admitted_sha256"] != row["expected_sha256"]
            or provenance
            != {
                "engine_id": row["engine_id"],
                "engine_version": row["engine_version"],
                "rules_version": row["rules_version"],
            }
        ):
            raise PrecomposedVectorError(f"corpus admission alias differs: {image_id}")


def _verify_corpus(
    corpus: Path,
    expected: dict[str, Any],
    repository: Path,
    corpus_admission: Any,
) -> dict[str, str]:
    package_name = expected.get("package")
    if not isinstance(package_name, str):
        raise PrecomposedVectorError("fixture package path is missing")
    package_path = _contained_regular_file(corpus, package_name, "fixture package")
    package_payload = package_path.read_bytes()
    package = _load_json(package_path)
    if (
        not isinstance(package, dict)
        or package.get("contract") != "typaxis.contract/1.4"
        or package.get("coordinate_unit") != "pdf_point_1_65536"
    ):
        raise PrecomposedVectorError("fixture package contract or coordinate unit differs")
    source_rows = package.get("sources")
    if not isinstance(source_rows, list) or not source_rows:
        raise PrecomposedVectorError("fixture package source ledger is missing")
    source_payloads: list[tuple[str, bytes]] = []
    for source_id, raw_source in enumerate(source_rows):
        source = _exact_object(
            raw_source,
            {"sha256", "source_id", "uri", "utf8_byte_length"},
            f"fixture source {source_id}",
        )
        if source["source_id"] != source_id or not isinstance(source["uri"], str):
            raise PrecomposedVectorError("fixture source IDs are not dense and ordered")
        payload = _contained_regular_file(
            corpus, source["uri"], f"fixture source {source_id}"
        ).read_bytes()
        try:
            payload.decode("utf-8")
        except UnicodeError as error:
            raise PrecomposedVectorError(
                f"fixture source {source_id} is not valid UTF-8"
            ) from error
        if (
            _integer(
                source["utf8_byte_length"],
                f"fixture source {source_id}.utf8_byte_length",
            )
            != len(payload)
            or _hash(source["sha256"], f"fixture source {source_id}.sha256")
            != _sha256(payload)
        ):
            raise PrecomposedVectorError(f"fixture source fact differs: {source_id}")
        source_payloads.append((source["uri"], payload))

    cases = _read_tsv(
        corpus / "cases.tsv",
        [
            "case_id", "kind", "image_id", "expected_sha256", "source_tex_path",
            "alt", "actual_text", "language", "advance", "ascent", "descent",
            "origin_x", "baseline", "viewport_width", "viewport_height",
            "spacing_before", "spacing_after", "equation_number", "minimum_gap",
            "categories",
        ],
    )
    case_ids = {row["case_id"] for row in cases}
    if case_ids != REQUIRED_CASES or len(case_ids) != len(cases):
        raise PrecomposedVectorError("combined corpus case set is incomplete or duplicated")
    if [row["case_id"] for row in cases] != sorted(case_ids):
        raise PrecomposedVectorError("combined corpus cases are not in canonical ID order")
    kinds = {row["kind"] for row in cases}
    if kinds != {"inline_vector", "math_vector", "vector_figure", "math_vector_block"}:
        raise PrecomposedVectorError("combined corpus does not cover all four vector kinds")
    for row in cases:
        categories = row["categories"].split(",")
        expected_categories = REQUIRED_CASE_CATEGORIES[row["case_id"]]
        if categories != sorted(set(categories)) or set(categories) != expected_categories:
            raise PrecomposedVectorError(
                f"case categories differ: {row['case_id']}"
            )
    fragments = _read_tsv(
        corpus / "fragments.tsv",
        [
            "fragment_id", "text_path", "cases", "inline_remaining_width",
            "block_frame_width", "block_remaining_height", "next_empty_frame_height",
            "categories",
        ],
    )
    fragment_ids = {row["fragment_id"] for row in fragments}
    if fragment_ids != REQUIRED_FRAGMENTS or len(fragment_ids) != len(fragments):
        raise PrecomposedVectorError("combined corpus fragment set is incomplete or duplicated")
    for row in fragments:
        categories = row["categories"].split(",")
        expected_categories = REQUIRED_FRAGMENT_CATEGORIES[row["fragment_id"]]
        if categories != sorted(set(categories)) or set(categories) != expected_categories:
            raise PrecomposedVectorError(
                f"fragment categories differ: {row['fragment_id']}"
            )
    resources = _read_tsv(
        corpus / "resources.tsv",
        [
            "image_id", "media_type", "uri", "svg_path", "expected_sha256",
            "engine_id", "engine_version", "rules_version",
        ],
    )
    if [
        _tsv_integer(row["image_id"], "resource image_id", minimum=0)
        for row in resources
    ] != list(range(13)):
        raise PrecomposedVectorError("resource IDs are not dense 0..12")
    expected_resources = {
        row["uri"]: (row["bytes"], row["sha256"])
        for row in expected.get("resource_hashes", [])
    }
    for row in resources:
        if row["media_type"] != "svg-safe-2":
            raise PrecomposedVectorError("combined corpus contains a non-Safe-SVG 2 resource")
        svg_path = row["svg_path"]
        payload = _contained_regular_file(corpus, svg_path, svg_path).read_bytes()
        if _sha256(payload) != row["expected_sha256"]:
            raise PrecomposedVectorError(f"resource ledger hash mismatch: {svg_path}")
        if row["uri"] != f"math/sha256-{row['expected_sha256']}.svg":
            raise PrecomposedVectorError(f"resource URI is not hash-derived: {svg_path}")
        expected_engine = (
            "vmb.texToSvg.cache-replay" if row["image_id"] == "1" else "vmb.texToSvg"
        )
        if (
            row["engine_id"] != expected_engine
            or row["engine_version"] != "2026.09.0"
            or row["rules_version"] != "vmb.math-safe-svg/1"
        ):
            raise PrecomposedVectorError(
                f"resource conversion identity differs: {svg_path}"
            )
        expected_fact = expected_resources.get(svg_path)
        if expected_fact is None or expected_fact != (len(payload), _sha256(payload)):
            raise PrecomposedVectorError(f"expected resource fact mismatch: {svg_path}")
    if set(expected_resources) != {row["svg_path"] for row in resources}:
        raise PrecomposedVectorError("expected resource set differs from the resource ledger")
    package_resources = package.get("resources")
    package_images = (
        package_resources.get("images")
        if isinstance(package_resources, dict)
        else None
    )
    if not isinstance(package_images, list) or len(package_images) != 1:
        raise PrecomposedVectorError("fixture package must bind exactly one image resource")
    package_image = _exact_object(
        package_images[0],
        {"expected_sha256", "image_id", "media_type", "uri", "vector_provenance"},
        "fixture package image 0",
    )
    base_resource = resources[0]
    package_provenance = _exact_object(
        package_image["vector_provenance"],
        {"engine_id", "engine_version", "rules_version"},
        "fixture package image 0 provenance",
    )
    if (
        package_image["image_id"] != 0
        or package_image["media_type"] != base_resource["media_type"]
        or package_image["uri"] != base_resource["svg_path"]
        or package_image["expected_sha256"] != base_resource["expected_sha256"]
        or package_provenance
        != {
            "engine_id": base_resource["engine_id"],
            "engine_version": base_resource["engine_version"],
            "rules_version": base_resource["rules_version"],
        }
    ):
        raise PrecomposedVectorError("fixture package image binding differs")
    _verify_corpus_admission(corpus_admission, resources)

    resource_by_id = {row["image_id"]: row for row in resources}
    metrics_by_case: dict[str, dict[str, int]] = {}
    tex_payloads: dict[str, bytes] = {}
    for row in cases:
        label = f"case {row['case_id']}"
        resource = resource_by_id.get(row["image_id"])
        if resource is None or row["expected_sha256"] != resource["expected_sha256"]:
            raise PrecomposedVectorError(f"{label}: resource/hash binding differs")
        if row["alt"] == "-":
            raise PrecomposedVectorError(f"{label}: alternative text is missing")
        _accessible_text(row["alt"], f"{label}.alt")
        if row["actual_text"] != "-":
            _accessible_text(row["actual_text"], f"{label}.actual_text")
        expected_language = (
            "en"
            if row["case_id"]
            in {"generic-block-override", "generic-inline-override", "long-block", "similar"}
            else "inherit"
        )
        if row["language"] != expected_language:
            raise PrecomposedVectorError(f"{label}: language intent differs")
        is_math = row["kind"] in {"math_vector", "math_vector_block"}
        is_inline = row["kind"] in {"inline_vector", "math_vector"}
        if is_math:
            tex_path = row["source_tex_path"]
            if not tex_path.startswith("tex/") or not tex_path.endswith(".tex"):
                raise PrecomposedVectorError(f"{label}: source TeX path is missing")
            tex_payloads[tex_path] = _canonical_utf8_text_file(corpus, tex_path, label)
        elif row["source_tex_path"] != "-" or row["actual_text"] != "-":
            raise PrecomposedVectorError(f"{label}: generic vector carries math text")

        if row["kind"] == "vector_figure":
            if any(row[field] != "-" for field in (
                "advance", "ascent", "descent", "origin_x", "baseline"
            )):
                raise PrecomposedVectorError(f"{label}: Figure carries inline metrics")
            viewport_width = _tsv_integer(row["viewport_width"], label, minimum=1)
            viewport_height = _tsv_integer(row["viewport_height"], label, minimum=1)
        else:
            metric = {
                field: _tsv_integer(row[field], f"{label}.{field}")
                for field in (
                    "advance", "ascent", "descent", "origin_x", "baseline",
                    "viewport_width", "viewport_height",
                )
            }
            if (
                metric["advance"] <= 0
                or metric["ascent"] <= 0
                or metric["descent"] < 0
                or metric["viewport_width"] <= 0
                or metric["viewport_height"] <= 0
                or not 0 <= metric["baseline"] <= metric["viewport_height"]
                or metric["ascent"] < metric["baseline"]
                or metric["descent"] < metric["viewport_height"] - metric["baseline"]
            ):
                raise PrecomposedVectorError(f"{label}: metric containment relation differs")
            if not -(2**63) <= metric["origin_x"] + metric["viewport_width"] <= 2**63 - 1:
                raise PrecomposedVectorError(f"{label}: horizontal metric extent overflows")
            metrics_by_case[row["case_id"]] = metric
            viewport_width = metric["viewport_width"]
            viewport_height = metric["viewport_height"]
        if viewport_width <= 0 or viewport_height <= 0:
            raise PrecomposedVectorError(f"{label}: viewport is not positive")
        if is_inline:
            for field in ("spacing_before", "spacing_after"):
                _tsv_integer(row[field], f"{label}.{field}", minimum=0)
        elif row["spacing_before"] != "-" or row["spacing_after"] != "-":
            raise PrecomposedVectorError(f"{label}: block spacing bypasses typed style")
        if row["kind"] == "math_vector_block":
            paired = (row["equation_number"] == "-") == (row["minimum_gap"] == "-")
            if not paired:
                raise PrecomposedVectorError(f"{label}: equation number binding is incomplete")
            if row["minimum_gap"] != "-":
                _tsv_integer(row["minimum_gap"], f"{label}.minimum_gap", minimum=1)
                _accessible_text(row["equation_number"], f"{label}.equation_number")
        elif row["equation_number"] != "-" or row["minimum_gap"] != "-":
            raise PrecomposedVectorError(f"{label}: equation number belongs to block math only")
    aliases = [row for row in resources if row["image_id"] in {"0", "1"}]
    if (
        len(aliases) != 2
        or aliases[0]["expected_sha256"] != aliases[1]["expected_sha256"]
        or aliases[0]["engine_id"] == aliases[1]["engine_id"]
    ):
        raise PrecomposedVectorError("cross-ID same-content alias provenance is not preserved")

    referenced_cases: set[str] = set()
    fragment_by_id = {row["fragment_id"]: row for row in fragments}
    fragment_payloads: dict[str, bytes] = {}
    for row in fragments:
        payload = _canonical_utf8_text_file(
            corpus, row["text_path"], row["fragment_id"]
        )
        fragment_payloads[row["text_path"]] = payload
        text = payload.decode("utf-8")
        occurrences = row["cases"].split(",")
        markers = re.findall(r"\{([a-z0-9]+(?:-[a-z0-9]+)*)\}", text)
        if markers != occurrences or any(case not in case_ids for case in occurrences):
            raise PrecomposedVectorError(
                f"fragment marker order differs: {row['fragment_id']}"
            )
        referenced_cases.update(occurrences)
    if referenced_cases != case_ids:
        raise PrecomposedVectorError("not every combined corpus case is placed in a fragment")
    line_end = fragment_by_id["line-end"]
    line_metric = metrics_by_case["x-plus-y"]
    line_case = next(row for row in cases if row["case_id"] == "x-plus-y")
    if _tsv_integer(line_end["inline_remaining_width"], "line-end") != (
        line_metric["advance"] + _tsv_integer(line_case["spacing_before"], "x-plus-y")
    ) or _tsv_integer(line_case["spacing_after"], "x-plus-y", minimum=1) <= 0:
        raise PrecomposedVectorError("line-end occupancy does not suppress trailing spacing")
    page_end = fragment_by_id["block-page-end"]
    frame_width = _tsv_integer(page_end["block_frame_width"], "block-page-end", minimum=1)
    remaining = _tsv_integer(page_end["block_remaining_height"], "block-page-end", minimum=0)
    next_height = _tsv_integer(page_end["next_empty_frame_height"], "block-page-end", minimum=1)
    long_metric = metrics_by_case["long-block"]
    if not (
        long_metric["viewport_width"] < frame_width
        and long_metric["viewport_width"] * 100 >= frame_width * 90
        and remaining < long_metric["viewport_height"] <= next_height
    ):
        raise PrecomposedVectorError("page-end block move context differs")
    mixed = fragment_by_id["mixed-heights"]["cases"].split(",")
    distinct_line_metrics = {
        (metrics_by_case[case]["ascent"], metrics_by_case[case]["descent"])
        for case in mixed
    }
    if len(distinct_line_metrics) < 3:
        raise PrecomposedVectorError("mixed-height line lacks varied ascent/descent metrics")
    negatives = _read_tsv(
        corpus / "negative-integration.tsv",
        [
            "case_id", "expected_phase", "expected_code", "location", "package_read",
            "resource_opened", "layout_started", "pdf_started", "visible_artifacts",
            "owner_test",
        ],
    )
    negative_ids = [row["case_id"] for row in negatives]
    if (
        negative_ids != sorted(REQUIRED_NEGATIVE_OUTCOMES)
        or set(negative_ids) != set(REQUIRED_NEGATIVE_OUTCOMES)
    ):
        raise PrecomposedVectorError(
            "negative integration ledger is incomplete, duplicated, or unordered"
        )
    allowed_phases = {
        "wire", "syntax", "profile", "style", "resource", "binding", "layout",
        "content-form", "structure", "pdf", "manifest",
    }
    for row in negatives:
        expected_phase, expected_code = REQUIRED_NEGATIVE_OUTCOMES[row["case_id"]]
        if (row["expected_phase"], row["expected_code"]) != (
            expected_phase,
            expected_code,
        ):
            raise PrecomposedVectorError(
                f"negative case outcome differs: {row['case_id']}"
            )
        if row["expected_phase"] not in allowed_phases:
            raise PrecomposedVectorError(
                f"negative case has an unknown phase: {row['case_id']}"
            )
        if not re.fullmatch(r"[A-Z][0-9]{4}", row["expected_code"]):
            raise PrecomposedVectorError(
                f"negative case has a noncanonical code: {row['case_id']}"
            )
        for field in ("package_read", "resource_opened", "layout_started", "pdf_started"):
            if row[field] not in {"true", "false"}:
                raise PrecomposedVectorError(
                    f"negative case has a nonboolean {field}: {row['case_id']}"
                )
        resource_opened = row["resource_opened"] == "true"
        layout_started = row["layout_started"] == "true"
        pdf_started = row["pdf_started"] == "true"
        if row["package_read"] != "true" or row["visible_artifacts"] != "diagnostics,manifest":
            raise PrecomposedVectorError(
                f"negative case violates the failed-publication policy: {row['case_id']}"
            )
        phase = row["expected_phase"]
        if phase in {"wire", "syntax", "profile", "style"}:
            expected_side_effects = (False, False, False)
        elif phase in {"resource", "binding"}:
            expected_side_effects = (True, False, False)
        elif phase in {"layout", "content-form", "structure"}:
            expected_side_effects = (True, True, False)
        else:
            expected_side_effects = (True, True, True)
        if (resource_opened, layout_started, pdf_started) != expected_side_effects:
            raise PrecomposedVectorError(
                f"negative case has impossible phase side effects: {row['case_id']}"
            )
    rust_test_names = _nonignored_rust_test_names(repository)
    missing_owners = sorted(
        {row["owner_test"] for row in negatives} - rust_test_names
    )
    if missing_owners:
        raise PrecomposedVectorError(
            f"negative ledger names missing executable Rust owners: {missing_owners}"
        )
    trace = _read_tsv(
        corpus / "assertion-traceability.tsv",
        ["assertion_id", "design_section", "fixture_or_artifact", "test_or_check"],
    )
    sections = {row["design_section"] for row in trace}
    if sections != {"15.1", "15.2", "15.3", "15.4", "15.5"}:
        raise PrecomposedVectorError(
            "design assertions are not traceable through sections 15.1..15.5"
        )
    assertion_ids = [row["assertion_id"] for row in trace]
    if assertion_ids != sorted(REQUIRED_ASSERTIONS) or set(assertion_ids) != REQUIRED_ASSERTIONS:
        raise PrecomposedVectorError(
            "design assertion IDs are incomplete, duplicated, or unordered"
        )
    generated_or_corpus_files = EXPECTED_ARTIFACTS | {
        "artifact-index.json",
        "negative-integration.tsv",
        "verify_reproducibility.py",
    }
    for row in trace:
        if row["fixture_or_artifact"] != REQUIRED_ASSERTION_REFERENCES[
            row["assertion_id"]
        ]:
            raise PrecomposedVectorError(
                f"assertion fixture mapping differs: {row['assertion_id']}"
            )
        if not row["assertion_id"].startswith(f"PV-{row['design_section']}-"):
            raise PrecomposedVectorError(
                f"assertion section differs: {row['assertion_id']}"
            )
        if row["test_or_check"] != REQUIRED_ASSERTION_CHECKS[row["assertion_id"]]:
            raise PrecomposedVectorError(
                f"assertion check mapping differs: {row['assertion_id']}"
            )
        context: str | None = None
        for reference in row["fixture_or_artifact"].split(","):
            if "#" in reference:
                context, member = reference.split("#", 1)
                if context == "cases.tsv":
                    known = case_ids
                elif context == "fragments.tsv":
                    known = fragment_ids
                else:
                    known = set()
                if member not in known:
                    raise PrecomposedVectorError(
                        f"assertion reference differs: {row['assertion_id']}"
                    )
            elif context in {"cases.tsv", "fragments.tsv"}:
                known = case_ids if context == "cases.tsv" else fragment_ids
                if reference not in known:
                    raise PrecomposedVectorError(
                        f"assertion reference differs: {row['assertion_id']}"
                    )
            elif reference not in generated_or_corpus_files:
                raise PrecomposedVectorError(
                    f"assertion artifact differs: {row['assertion_id']}"
                )
    return {
        "assertion_traceability_sha256": _sha256(
            (corpus / "assertion-traceability.tsv").read_bytes()
        ),
        "cases_sha256": _sha256((corpus / "cases.tsv").read_bytes()),
        "fragment_text_set_sha256": _artifact_set_digest(fragment_payloads.items()),
        "fragments_sha256": _sha256((corpus / "fragments.tsv").read_bytes()),
        "negative_sha256": _sha256((corpus / "negative-integration.tsv").read_bytes()),
        "package_sha256": _sha256(package_payload),
        "resources_sha256": _sha256((corpus / "resources.tsv").read_bytes()),
        "source_set_sha256": _artifact_set_digest(source_payloads),
        "tex_set_sha256": _artifact_set_digest(tex_payloads.items()),
    }


def _normalize_corpus_text(value: str) -> str:
    output: list[str] = []
    pending_space = False
    for character in value:
        if ord(character) in _UNICODE_WHITE_SPACE:
            pending_space = bool(output)
        else:
            if pending_space:
                output.append(" ")
                pending_space = False
            output.append(character)
    return "".join(output)


def _corpus_document_expectation(
    corpus: Path,
) -> tuple[list[dict[str, Any]], str, list[tuple[str, str, str]]]:
    cases = _read_tsv(
        corpus / "cases.tsv",
        [
            "case_id", "kind", "image_id", "expected_sha256", "source_tex_path",
            "alt", "actual_text", "language", "advance", "ascent", "descent",
            "origin_x", "baseline", "viewport_width", "viewport_height",
            "spacing_before", "spacing_after", "equation_number", "minimum_gap",
            "categories",
        ],
    )
    cases_by_id = {row["case_id"]: row for row in cases}
    fragments = _read_tsv(
        corpus / "fragments.tsv",
        [
            "fragment_id", "text_path", "cases", "inline_remaining_width",
            "block_frame_width", "block_remaining_height", "next_empty_frame_height",
            "categories",
        ],
    )
    placements: list[dict[str, Any]] = []
    extracted: list[str] = []
    marked_content: list[tuple[str, str, str]] = []
    usage_id = 0
    for page_index, fragment in enumerate(fragments):
        text = _canonical_utf8_text_file(
            corpus, fragment["text_path"], fragment["fragment_id"]
        ).decode("utf-8")
        cursor = 0
        markers = list(re.finditer(r"\{([a-z0-9]+(?:-[a-z0-9]+)*)\}", text))
        if [match.group(1) for match in markers] != fragment["cases"].split(","):
            raise PrecomposedVectorError(
                f"corpus document marker order differs: {fragment['fragment_id']}"
            )
        for paint_ordinal, match in enumerate(markers):
            preceding_text = text[cursor : match.start()]
            extracted.append(preceding_text)
            if preceding_text:
                marked_content.append(("Span", preceding_text, "ja"))
            case_id = match.group(1)
            case = cases_by_id[case_id]
            actual_text = case["alt"] if case["actual_text"] == "-" else case["actual_text"]
            extracted.append(actual_text)
            language = "ja" if case["language"] == "inherit" else case["language"]
            role = (
                "Formula"
                if case["kind"] in {"math_vector", "math_vector_block"}
                else "Figure"
            )
            marked_content.append((role, actual_text, language))
            placements.append(
                {
                    "actual_text": actual_text,
                    "baseline": (
                        None
                        if case["baseline"] == "-"
                        else _tsv_integer(case["baseline"], f"{case_id}.baseline")
                    ),
                    "case_id": case_id,
                    "expected_sha256": case["expected_sha256"],
                    "image_id": _tsv_integer(
                        case["image_id"], f"{case_id}.image_id", minimum=0
                    ),
                    "kind": case["kind"],
                    "language": language,
                    "origin_x": (
                        None
                        if case["origin_x"] == "-"
                        else _tsv_integer(case["origin_x"], f"{case_id}.origin_x")
                    ),
                    "page_index": page_index,
                    "paint_ordinal": paint_ordinal,
                    "usage_id": usage_id,
                    "viewport_height": _tsv_integer(
                        case["viewport_height"], f"{case_id}.viewport_height", minimum=1
                    ),
                    "viewport_width": _tsv_integer(
                        case["viewport_width"], f"{case_id}.viewport_width", minimum=1
                    ),
                }
            )
            usage_id += 1
            cursor = match.end()
        trailing_text = text[cursor:]
        extracted.append(trailing_text)
        if trailing_text:
            marked_content.append(("Span", trailing_text, "ja"))
    return placements, _normalize_corpus_text("".join(extracted)), marked_content


def _verify_corpus_pdf_and_display(
    directory: Path,
    payloads: dict[str, bytes],
    expected: Any,
    receipt: Any,
    admission: Any,
    corpus: Path,
) -> None:
    expected_value = _exact_object(
        expected,
        {
            "do_count",
            "form_count",
            "normalized_extracted_text",
            "page_count",
            "placement_count",
            "resource_count",
        },
        "fixture expected precomposed vector corpus",
    )
    expected_counts = {
        "do_count": 33,
        "form_count": 12,
        "page_count": 8,
        "placement_count": 33,
        "resource_count": 12,
    }
    if any(expected_value[name] != value for name, value in expected_counts.items()):
        raise PrecomposedVectorError("fixture expected corpus counts differ")
    if not isinstance(expected_value["normalized_extracted_text"], str):
        raise PrecomposedVectorError("fixture expected corpus text is not a string")

    receipt_value = _exact_object(
        receipt, {"do", "forms", "pages", "pdf_sha256"}, "corpus receipt"
    )
    pdf = payloads["corpus-output.pdf"]
    if receipt_value != {
        "do": expected_counts["do_count"],
        "forms": expected_counts["form_count"],
        "pages": expected_counts["page_count"],
        "pdf_sha256": _sha256(pdf),
    }:
        raise PrecomposedVectorError("corpus PDF receipt differs")
    objects = _verify_vector_pdf_shape(
        pdf,
        "corpus-output.pdf",
        expected_counts["form_count"],
        expected_counts["do_count"],
    )
    if sum(b"/Type /Page " in body for _, body in objects) != expected_counts["page_count"]:
        raise PrecomposedVectorError("corpus PDF page count differs")

    placements, derived_text, expected_marked_content = _corpus_document_expectation(corpus)
    if len(placements) != expected_counts["placement_count"]:
        raise PrecomposedVectorError("corpus occurrence count differs from fragments.tsv")
    if derived_text != expected_value["normalized_extracted_text"]:
        raise PrecomposedVectorError("corpus expected text differs from checked-in fragments")
    marked_content = _decode_marked_content(pdf)
    if marked_content != expected_marked_content:
        raise PrecomposedVectorError("corpus PDF role, ActualText, Lang, or order differs")
    if _normalize_corpus_text("".join(text for _, text, _ in marked_content)) != derived_text:
        raise PrecomposedVectorError("corpus PDF normalized extraction differs")
    if (
        sum(role == "Formula" for role, _, _ in marked_content) != 29
        or sum(role == "Figure" for role, _, _ in marked_content) != 4
    ):
        raise PrecomposedVectorError("corpus PDF Formula/Figure coverage differs")
    for source in sorted((corpus / "tex").glob("*.tex")):
        tex = source.read_bytes().strip()
        if tex and tex in pdf:
            raise PrecomposedVectorError(
                f"opaque source TeX leaked into corpus PDF: {source.name}"
            )

    outer = _exact_object(
        _load_json(directory / "corpus-display.json"),
        {"contract", "coordinate_unit", "precomposed_vector_display"},
        "corpus Display artifact",
    )
    if (
        outer["contract"] != "typaxis.contract/1.4"
        or outer["coordinate_unit"] != "pdf_point_1_65536"
    ):
        raise PrecomposedVectorError("corpus Display contract or coordinate unit differs")
    display = _exact_object(
        outer["precomposed_vector_display"],
        {
            "admitted_fingerprint",
            "algorithm",
            "binding_set_fingerprint",
            "block_selected_layout_fingerprint",
            "command_count",
            "content_key_count",
            "inline_selected_layout_fingerprint",
            "limits_fingerprint",
            "package_sha256",
            "page_count",
            "page_geometry_fingerprint",
            "pages",
            "profile_fingerprint",
        },
        "corpus Display",
    )
    if (
        display["algorithm"] != "typaxis.draw-vector-display/2"
        or display["page_count"] != expected_counts["page_count"]
        or display["command_count"] != expected_counts["placement_count"]
        or display["content_key_count"] != expected_counts["resource_count"]
        or not isinstance(display["pages"], list)
        or len(display["pages"]) != expected_counts["page_count"]
    ):
        raise PrecomposedVectorError("corpus Display counts or identity differ")
    commands: list[dict[str, Any]] = []
    for page_index, raw_page in enumerate(display["pages"]):
        page = _exact_object(raw_page, {"fingerprint", "record"}, "corpus Display page")
        record = _exact_object(
            page["record"], {"commands", "page_index"}, "corpus Display page record"
        )
        if (
            record["page_index"] != page_index
            or not isinstance(record["commands"], list)
            or _hash(page["fingerprint"], "corpus Display page fingerprint")
            != _sha256(canonical_json_bytes(record))
        ):
            raise PrecomposedVectorError("corpus Display page order or commands differ")
        commands.extend(record["commands"])
    if len(commands) != len(placements):
        raise PrecomposedVectorError("corpus Display occurrence count differs")

    common_command_keys = {
        "binding_fingerprint",
        "content_key",
        "fragment_ordinal",
        "frame_index",
        "image_id",
        "ir_fingerprint",
        "kind",
        "matrix",
        "op",
        "owner",
        "page_index",
        "paint_ordinal",
        "resolved_current_color",
        "scale",
        "selected_placement_fingerprint",
        "usage_id",
        "viewport",
    }
    admission_candidates = admission.get("candidates") if isinstance(admission, dict) else None
    if not isinstance(admission_candidates, list):
        raise PrecomposedVectorError("corpus admission candidates are malformed")
    admitted_keys: dict[str, dict[str, Any]] = {}
    for offset, candidate in enumerate(admission_candidates):
        if not isinstance(candidate, dict):
            raise PrecomposedVectorError("corpus admission candidate is malformed")
        key = _exact_object(
            candidate.get("key"),
            {"ir_fingerprint", "ir_id", "media_type", "parser_id", "source_sha256"},
            f"corpus admitted content key {offset}",
        )
        source_hash = _hash(key["source_sha256"], "corpus admitted source hash")
        if source_hash in admitted_keys:
            raise PrecomposedVectorError("corpus admission duplicates a source content key")
        admitted_keys[source_hash] = key

    content_keys: set[bytes] = set()
    next_math_flow_id = 0
    for expected_placement, raw_command in zip(placements, commands, strict=True):
        wrapped = _exact_object(
            raw_command, {"fingerprint", "record"}, "corpus Display command"
        )
        command_keys = set(common_command_keys)
        if expected_placement["kind"] == "vector_figure":
            command_keys.add("figure_caption")
        else:
            command_keys.add("baseline_metrics")
        if expected_placement["kind"] == "math_vector_block":
            command_keys.add("math_flow")
        command = _exact_object(
            wrapped["record"],
            command_keys,
            f"corpus usage {expected_placement['usage_id']} command",
        )
        if _hash(
            wrapped["fingerprint"],
            f"corpus usage {expected_placement['usage_id']} fingerprint",
        ) != _sha256(canonical_json_bytes(command)):
            raise PrecomposedVectorError("corpus Display command fingerprint differs")
        viewport = _rect_as_list(
            command.get("viewport"),
            f"corpus usage {expected_placement['usage_id']} viewport",
        )
        matrix = _matrix_as_list(
            command.get("matrix"),
            f"corpus usage {expected_placement['usage_id']} matrix",
        )
        content_key = _exact_object(
            command.get("content_key"),
            {"ir_fingerprint", "ir_id", "media_type", "parser_id", "source_sha256"},
            f"corpus usage {expected_placement['usage_id']} content key",
        )
        expected_identity = {
            "image_id": expected_placement["image_id"],
            "kind": expected_placement["kind"],
            "page_index": expected_placement["page_index"],
            "paint_ordinal": expected_placement["paint_ordinal"],
            "usage_id": expected_placement["usage_id"],
        }
        if any(command.get(name) != value for name, value in expected_identity.items()):
            raise PrecomposedVectorError(
                f"corpus Display occurrence identity differs: {expected_placement['case_id']}"
            )
        color = _exact_object(
            command.get("resolved_current_color"),
            {"blue", "green", "red"},
            f"corpus usage {expected_placement['usage_id']} color",
        )
        expected_binding = _sha256(
            f"typaxis.precomposed-vector-corpus-binding/{expected_placement['usage_id']}".encode()
        )
        expected_selected = _sha256(
            f"typaxis.precomposed-vector-corpus-selected/{expected_placement['usage_id']}".encode()
        )
        if (
            command.get("op") != "draw_vector"
            or command.get("owner") != 1000 + expected_placement["usage_id"]
            or command.get("frame_index") != 0
            or command.get("fragment_ordinal") != 0
            or command.get("binding_fingerprint") != expected_binding
            or command.get("selected_placement_fingerprint") != expected_selected
            or command.get("scale") != 65_536
            or matrix != [65_536, 0, 0, 65_536, viewport[0], viewport[1]]
            or viewport[2] != expected_placement["viewport_width"]
            or viewport[3] != expected_placement["viewport_height"]
            or content_key["source_sha256"] != expected_placement["expected_sha256"]
            or content_key
            != admitted_keys.get(expected_placement["expected_sha256"])
            or content_key["media_type"] != "svg-safe-2"
            or content_key["parser_id"] != "typaxis.safe-svg-parser/2"
            or content_key["ir_id"] != "typaxis.safe-vector-ir/2"
            or command.get("ir_fingerprint") != content_key["ir_fingerprint"]
            or color != {"blue": 0, "green": 0, "red": 0}
        ):
            raise PrecomposedVectorError(
                f"corpus Display placement/content closure differs: {expected_placement['case_id']}"
            )
        content_keys.add(canonical_json_bytes(content_key))
        baseline = command.get("baseline_metrics")
        if expected_placement["baseline"] is None:
            if baseline is not None:
                raise PrecomposedVectorError("corpus Figure unexpectedly has baseline metrics")
            caption = _exact_object(
                command.get("figure_caption"),
                {"caption_flow_id", "caption_owners", "keep_caption"},
                f"corpus usage {expected_placement['usage_id']} Figure caption",
            )
            if caption != {
                "caption_flow_id": 0,
                "caption_owners": [],
                "keep_caption": False,
            }:
                raise PrecomposedVectorError("corpus Figure retained a template caption")
        else:
            baseline_value = _exact_object(
                baseline,
                {
                    "baseline",
                    "baseline_y",
                    "metric_receipt_fingerprint",
                    "pen_origin_x",
                },
                f"corpus usage {expected_placement['usage_id']} baseline",
            )
            if (
                baseline_value["baseline"] != expected_placement["baseline"]
                or baseline_value["baseline_y"]
                != viewport[1] + expected_placement["baseline"]
                or baseline_value["pen_origin_x"] + expected_placement["origin_x"]
                != viewport[0]
                or baseline_value["metric_receipt_fingerprint"]
                != _sha256(
                    f"typaxis.precomposed-vector-corpus-metrics/{expected_placement['usage_id']}".encode()
                )
            ):
                raise PrecomposedVectorError(
                    f"corpus baseline equation differs: {expected_placement['case_id']}"
                )
        if expected_placement["kind"] == "math_vector_block":
            math_flow = _exact_object(
                command.get("math_flow"),
                {
                    "flow_fingerprint",
                    "flow_id",
                    "parent_flow_id",
                    "parent_position",
                    "terminal",
                    "terminal_receipt_fingerprint",
                },
                f"corpus usage {expected_placement['usage_id']} math flow",
            )
            usage_id = expected_placement["usage_id"]
            if math_flow != {
                "flow_fingerprint": _sha256(
                    f"typaxis.precomposed-vector-corpus-flow/{usage_id}".encode()
                ),
                "flow_id": next_math_flow_id,
                "parent_flow_id": 0,
                "parent_position": expected_placement["paint_ordinal"],
                "terminal": 1,
                "terminal_receipt_fingerprint": _sha256(
                    f"typaxis.precomposed-vector-corpus-terminal/{usage_id}".encode()
                ),
            }:
                raise PrecomposedVectorError("corpus math-flow relation differs")
            next_math_flow_id += 1
    if len(content_keys) != expected_counts["resource_count"]:
        raise PrecomposedVectorError("corpus Display content-key dedupe differs")


def _verify_effective_document_package(
    directory: Path,
    payloads: dict[str, bytes],
    verification: dict[str, Any],
    safe: dict[str, Any],
    math: dict[str, Any],
    tagged: dict[str, Any],
    book: dict[str, Any],
    corpus: Path,
    repository: Path,
) -> None:
    payload = payloads["effective-document-package.json"]
    package = _load_json(directory / "effective-document-package.json")
    effective_hash = _sha256(payload[:-1])
    if _hash(
        verification["effective_package_sha256"], "effective package receipt"
    ) != effective_hash:
        raise PrecomposedVectorError("effective package receipt hash differs")
    safe_fingerprints = safe.get("fingerprints")
    math_fingerprints = math.get("fingerprints")
    tagged_fingerprints = tagged.get("fingerprints")
    book_fingerprints = book.get("fingerprints")
    if not all(
        isinstance(value, dict)
        for value in (
            safe_fingerprints,
            math_fingerprints,
            tagged_fingerprints,
            book_fingerprints,
        )
    ):
        raise PrecomposedVectorError("manifest package fingerprints are malformed")
    if (
        tagged_fingerprints.get("package_sha256") != effective_hash
        or book_fingerprints.get("package_sha256") != effective_hash
        or safe_fingerprints.get("package_sha256")
        != math_fingerprints.get("package_sha256")
        or safe_fingerprints.get("package_sha256")
        != book_fingerprints.get("semantic_sha256")
    ):
        raise PrecomposedVectorError("effective/semantic package manifest closure differs")
    if (
        not isinstance(package, dict)
        or package.get("contract") != "typaxis.contract/1.4"
        or package.get("coordinate_unit") != "pdf_point_1_65536"
    ):
        raise PrecomposedVectorError("effective package contract or coordinate unit differs")

    sources = package.get("sources")
    if not isinstance(sources, list) or len(sources) != 1:
        raise PrecomposedVectorError("effective package source ledger differs")
    source = _exact_object(
        sources[0], {"sha256", "source_id", "uri", "utf8_byte_length"}, "effective source"
    )
    source_payload = _contained_regular_file(corpus, source["uri"], "effective source").read_bytes()
    if (
        source["source_id"] != 0
        or source["sha256"] != _sha256(source_payload)
        or source["utf8_byte_length"] != len(source_payload)
    ):
        raise PrecomposedVectorError("effective package source facts differ")

    resources = package.get("resources")
    if not isinstance(resources, dict):
        raise PrecomposedVectorError("effective package resources are malformed")
    images = resources.get("images")
    fonts = resources.get("font_faces")
    if not isinstance(images, list) or not isinstance(fonts, list):
        raise PrecomposedVectorError("effective package resource arrays are malformed")
    corpus_resources = _read_tsv(
        corpus / "resources.tsv",
        [
            "image_id", "media_type", "uri", "svg_path", "expected_sha256",
            "engine_id", "engine_version", "rules_version",
        ],
    )
    resources_by_path: dict[str, list[dict[str, str]]] = {}
    for row in corpus_resources:
        resources_by_path.setdefault(row["svg_path"], []).append(row)
    if [image.get("image_id") for image in images] != [0, 1]:
        raise PrecomposedVectorError("effective package image IDs differ")
    for image in images:
        value = _exact_object(
            image,
            {"expected_sha256", "image_id", "media_type", "uri", "vector_provenance"},
            "effective package image",
        )
        provenance = _exact_object(
            value["vector_provenance"],
            {"engine_id", "engine_version", "rules_version"},
            "effective package image provenance",
        )
        resource_payload = _contained_regular_file(
            corpus, value["uri"], "effective package image"
        ).read_bytes()
        if (
            value["media_type"] != "svg-safe-2"
            or value["expected_sha256"] != _sha256(resource_payload)
            or not any(
                value["expected_sha256"] == row["expected_sha256"]
                and provenance
                == {
                    "engine_id": row["engine_id"],
                    "engine_version": row["engine_version"],
                    "rules_version": row["rules_version"],
                }
                for row in resources_by_path.get(value["uri"], [])
            )
        ):
            raise PrecomposedVectorError("effective package image facts differ")
    if len(fonts) != 1:
        raise PrecomposedVectorError("effective package font ledger differs")
    font = _exact_object(
        fonts[0],
        {"expected_sha256", "face_index", "family", "font_face_id", "media_type", "uri"},
        "effective package font",
    )
    font_path = repository / "samples/machine-package/staging/production-book-1/math/job/math.ttf"
    font_payload = font_path.read_bytes()
    if font != {
        "expected_sha256": _sha256(font_payload),
        "face_index": 0,
        "family": "Math",
        "font_face_id": 0,
        "media_type": "sfnt-truetype-glyf",
        "uri": "math.ttf",
    }:
        raise PrecomposedVectorError("effective package font facts differ")

    kinds: list[str] = []
    pending: list[Any] = [package.get("document")]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            kind = value.get("kind")
            if isinstance(kind, str):
                kinds.append(kind)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    if not {
        "inline_vector",
        "math_vector",
        "vector_figure",
        "math_vector_block",
    }.issubset(kinds):
        raise PrecomposedVectorError("effective package does not contain all four vector kinds")


def verify_artifacts(
    artifact_directory: Path,
    expectation_path: Path,
    repository: Path,
    *,
    require_external_tools: bool = False,
    readiness_directory: Path | None = None,
    mutool: str | None = None,
    pdftotext: str | None = None,
    pdfinfo: str | None = None,
    verapdf: str | None = None,
    binary: str | None = None,
) -> dict[str, Any]:
    directory = artifact_directory.resolve(strict=True)
    if not directory.is_dir():
        raise PrecomposedVectorError("artifact path is not a directory")
    expectation = _exact_object(
        _load_json(expectation_path.resolve(strict=True)),
        {
            "advertised_item_coverage", "arguments", "command", "contract", "expected",
            "fixture_class", "fixture_id", "package", "profile", "resource_hashes",
        },
        "fixture expectation",
    )
    if (
        expectation["contract"] != "typaxis.contract/1.4"
        or expectation["command"] != "private-production-book"
        or expectation["profile"] != "typaxis.machine-pdf/production-book-1"
        or expectation["fixture_id"] != FIXTURE_ID
        or expectation["fixture_class"] != "positive"
        or expectation["package"] != "document-package.json"
        or expectation["arguments"] != ["precomposed-vector", "--output", "$OUTPUT"]
        or expectation["advertised_item_coverage"] != EXPECTED_COVERAGE
    ):
        raise PrecomposedVectorError("fixture expectation is not the private MI4-V18 contract")
    expected_outcome = _exact_object(
        expectation["expected"],
        {
            "exit_code",
            "location",
            "manifest_progress",
            "normalized_extracted_text",
            "page_count",
            "precomposed_vector_corpus",
            "primary_code",
            "side_effects",
            "visible_artifacts",
        },
        "fixture expected outcome",
    )
    if (
        expected_outcome["exit_code"] != 0
        or expected_outcome["location"] is not None
        or expected_outcome["primary_code"] is not None
        or expected_outcome["manifest_progress"]
        != {"package": "validated", "resources": "admitted", "sources": "admitted"}
        or expected_outcome["side_effects"]
        != {
            "layout_started": True,
            "package_read": True,
            "pdf_started": True,
            "resource_opened": True,
            "source_read": True,
        }
        or expected_outcome["visible_artifacts"] != ["manifest", "pdf", "trace"]
    ):
        raise PrecomposedVectorError("fixture expected outcome closure differs")
    records, payloads = _artifact_index(directory)
    verification = _exact_object(
        _load_json(directory / "verification.json"),
        {
            "alias_use",
            "contract",
            "corpus",
            "counts",
            "effective_package_sha256",
            "figure",
            "pdf_sha256",
            "ten_use",
        },
        "verification receipt",
    )
    if verification["contract"] != "typaxis.private-precomposed-vector-verification/1":
        raise PrecomposedVectorError("verification receipt contract differs")
    counts = _exact_object(
        verification["counts"],
        {"do", "forms", "math_facts", "objects", "pages", "placements", "resources", "structures"},
        "verification counts",
    )
    expected_counts = {
        "do": 4,
        "forms": 1,
        "math_facts": 2,
        "objects": 29,
        "pages": 2,
        "placements": 4,
        "resources": 2,
        "structures": 4,
    }
    normalized_counts = {
        name: _integer(value, f"counts.{name}") for name, value in counts.items()
    }
    if (
        normalized_counts != expected_counts
        or normalized_counts["pages"] != expected_outcome["page_count"]
    ):
        raise PrecomposedVectorError("expected and generated counts differ")
    if verification["pdf_sha256"] != _sha256(payloads["output.pdf"]):
        raise PrecomposedVectorError("verification receipt PDF hash differs")
    extracted = _verify_pdf(payloads["output.pdf"], counts, expectation_path.parent)
    try:
        structure_expectation = pdf_structure.load_expectation(
            directory / "tagged-pdf-expectation.json"
        )
        structure_result = pdf_structure.verify_tagged_pdf_structure_v2(
            payloads["output.pdf"], structure_expectation
        )
    except pdf_structure.PdfValidationError as error:
        raise PrecomposedVectorError(
            f"independent tagged-PDF /2 validation failed: {error}"
        ) from error
    if (
        structure_result.get("pdf_sha256") != verification["pdf_sha256"]
        or structure_result.get("page_count") != normalized_counts["pages"]
        or structure_result.get("form_count") != normalized_counts["forms"]
        or structure_result.get("vector_count") != normalized_counts["placements"]
        or structure_result.get("equation_number_count") != 1
        or structure_result.get("extracted_text") != ["xたすy", "xたすy、式1", "(1)"]
    ):
        raise PrecomposedVectorError(
            "independent tagged-PDF /2 observation differs from the fixture"
        )
    normalized = " ".join(extracted)
    if normalized != expected_outcome["normalized_extracted_text"]:
        raise PrecomposedVectorError(
            f"normalized ActualText differs: {normalized!r}"
        )
    ten = _exact_object(verification["ten_use"], {"do", "forms", "pdf_sha256"}, "ten_use")
    ten_pdf = payloads["dedupe-ten-use.pdf"]
    if (
        ten != {"do": 10, "forms": 1, "pdf_sha256": _sha256(ten_pdf)}
        or ten_pdf.count(b"/Subtype /Form") != 1
        or len(_DO.findall(ten_pdf)) != 10
        or b"/Subtype /Image" in ten_pdf
    ):
        raise PrecomposedVectorError("ten-use fixture is not exactly one Form and ten Do operators")
    _verify_vector_pdf_shape(ten_pdf, "dedupe-ten-use.pdf", 1, 10)

    alias = _exact_object(
        verification["alias_use"],
        {"aliases", "do", "forms", "pdf_sha256", "provenance_facts"},
        "alias_use",
    )
    alias_pdf = payloads["dedupe-two-alias.pdf"]
    if (
        alias != {
            "aliases": 2,
            "do": 4,
            "forms": 1,
            "pdf_sha256": _sha256(alias_pdf),
            "provenance_facts": 2,
        }
        or alias_pdf.count(b"/Subtype /Form") != 1
        or len(_DO.findall(alias_pdf)) != 4
        or b"/Subtype /Image" in alias_pdf
    ):
        raise PrecomposedVectorError(
            "cross-ID alias fixture is not one Form with two provenance facts"
        )
    _verify_vector_pdf_shape(alias_pdf, "dedupe-two-alias.pdf", 1, 4)

    figure = _exact_object(
        verification["figure"],
        {"do", "forms", "pdf_sha256", "structures"},
        "figure",
    )
    figure_pdf = payloads["figure-output.pdf"]
    if figure != {
        "do": 1,
        "forms": 1,
        "pdf_sha256": _sha256(figure_pdf),
        "structures": 1,
    }:
        raise PrecomposedVectorError("legacy Figure verification receipt differs")
    _verify_vector_pdf_shape(figure_pdf, "figure-output.pdf", 1, 1)
    if (
        figure_pdf.count(b"/S /Figure") != 1
        or b"/S /Formula" in figure_pdf
        or b"/Alt <FEFF" not in figure_pdf
    ):
        raise PrecomposedVectorError("legacy Figure PDF semantics differ")

    inline = _load_json(directory / "inline-layout-trace.json")["precomposed_vector_layout"]
    safe = _load_json(directory / "safe-vector-manifest.json")
    display = _load_json(directory / "display-v2.json")["precomposed_vector_display"]
    block = _load_json(directory / "block-layout-trace.json")["precomposed_vector_block_layout"]
    math = _load_json(directory / "math-vector-manifest.json")
    tagged = _load_json(directory / "tagged-pdf-manifest.json")
    book = _load_json(directory / "book-navigation-manifest.json")
    observation = _load_json(directory / "pdf-observation.json")
    corpus_admission = _load_json(directory / "corpus-admission.json")
    root = _load_json(directory / "build-manifest-vector.json")
    figure_root = _load_json(directory / "figure-build-manifest-vector.json")
    phases = _load_json(directory / "phase-receipts.json")
    if phases != {
        "contract": "typaxis.private-production-phase-receipts/1",
        "phases": EXPECTED_PHASES,
        "status": "built",
    }:
        raise PrecomposedVectorError("private production phases are missing or reordered")

    resources = safe.get("resources")
    if not isinstance(resources, list) or len(resources) != counts["resources"]:
        raise PrecomposedVectorError("SafeVector resource count differs")
    manifest_placements: list[dict[str, Any]] = []
    content_key_by_node: dict[int, Any] = {}
    for resource_index, resource in enumerate(resources):
        if not isinstance(resource, dict) or not isinstance(resource.get("placements"), list):
            raise PrecomposedVectorError(
                f"SafeVector resource {resource_index} placements are malformed"
            )
        placements = resource["placements"]
        if not all(isinstance(placement, dict) for placement in placements):
            raise PrecomposedVectorError(
                f"SafeVector resource {resource_index} placement is malformed"
            )
        if resource.get("total_placement_count") != len(placements):
            raise PrecomposedVectorError(
                f"SafeVector resource {resource_index} placement count differs"
            )
        manifest_placements.extend(placements)
        for placement in placements:
            node = placement.get("node_id")
            if isinstance(node, int) and not isinstance(node, bool):
                content_key_by_node[node] = resource.get("content_key")
    manifest_nodes = [placement.get("node_id") for placement in manifest_placements]
    usage_ids = [placement.get("usage_id") for placement in manifest_placements]
    semantic_fragments = [
        placement.get("fragment_ordinal") for placement in manifest_placements
    ]
    if (
        manifest_nodes != [3, 4, 5, 6]
        or usage_ids != list(range(4))
        or semantic_fragments != [0, 0, 0, 0]
    ):
        raise PrecomposedVectorError("SafeVector placements are missing, duplicated, or reordered")
    manifest_by_node = {
        placement["node_id"]: placement for placement in manifest_placements
    }

    inline_entries = inline.get("placements")
    line_entries = inline.get("lines")
    if (
        not isinstance(inline_entries, list)
        or not isinstance(line_entries, list)
        or inline.get("placement_count") != len(inline_entries)
        or len(inline_entries) != 2
        or inline.get("line_count") != len(line_entries)
        or len(line_entries) != 1
    ):
        raise PrecomposedVectorError("inline trace count closure differs")
    if not all(
        isinstance(entry, dict) and isinstance(entry.get("record"), dict)
        for entry in inline_entries
    ):
        raise PrecomposedVectorError("inline trace placement is malformed")
    inline_nodes = [entry.get("record", {}).get("node_id") for entry in inline_entries]
    if inline_nodes != [3, 4]:
        raise PrecomposedVectorError("inline trace nodes are missing, duplicated, or reordered")
    trace_by_node: dict[int, dict[str, Any]] = {}
    for occurrence, entry in enumerate(inline_entries):
        trace_placement = entry["record"]
        node = trace_placement["node_id"]
        manifest_placement = manifest_by_node[node]
        viewport = _rect_as_list(trace_placement.get("viewport"), f"inline node {node} viewport")
        metrics = manifest_placement.get("metrics")
        spacing = trace_placement.get("spacing")
        if not isinstance(metrics, dict) or not isinstance(spacing, dict):
            raise PrecomposedVectorError(f"inline node {node} metrics or spacing are malformed")
        if trace_placement.get("baseline_y") != viewport[1] + trace_placement.get(
            "baseline", -1
        ):
            raise PrecomposedVectorError(
                f"inline baseline equation failed at node {node}"
            )
        expected_matrix = [
            trace_placement.get("scale"),
            0,
            0,
            trace_placement.get("scale"),
            viewport[0],
            viewport[1],
        ]
        if (
            trace_placement.get("occurrence") != occurrence
            or viewport[0]
            != trace_placement.get("pen_origin_x", -1) + metrics.get("origin_x", 0)
            or manifest_placement.get("binding_fingerprint")
            != trace_placement.get("binding_fingerprint")
            or manifest_placement.get("selected_placement_fingerprint")
            != entry.get("fingerprint")
            or manifest_placement.get("metrics", {}).get("baseline")
            != trace_placement.get("baseline")
            or manifest_placement.get("viewport") != viewport
            or manifest_placement.get("matrix") != expected_matrix
            or manifest_placement.get("scale") != trace_placement.get("scale")
            or manifest_placement.get("spacing_before") != spacing.get("before")
            or manifest_placement.get("spacing_after") != spacing.get("after")
        ):
            raise PrecomposedVectorError(
                f"inline trace/manifest placement closure failed at node {node}"
            )
        trace_by_node[node] = trace_placement
    line = line_entries[0].get("record") if isinstance(line_entries[0], dict) else None
    if not isinstance(line, dict):
        raise PrecomposedVectorError("inline line trace is malformed")
    inline_metrics = [manifest_by_node[node]["metrics"] for node in inline_nodes]
    if line.get("content_ascent") != max(
        metric["ascent"] for metric in inline_metrics
    ):
        raise PrecomposedVectorError("line ascent is not the maximum inline ascent")
    if line.get("content_descent") != max(
        metric["descent"] for metric in inline_metrics
    ):
        raise PrecomposedVectorError("line descent is not the maximum inline descent")

    block_entries = block.get("block_placements")
    if (
        not isinstance(block_entries, list)
        or block.get("block_placement_count") != len(block_entries)
        or len(block_entries) != 2
    ):
        raise PrecomposedVectorError("block trace count closure differs")
    if not all(
        isinstance(entry, dict) and isinstance(entry.get("record"), dict)
        for entry in block_entries
    ):
        raise PrecomposedVectorError("block trace placement is malformed")
    block_nodes = [entry.get("record", {}).get("node_id") for entry in block_entries]
    if block_nodes != [5, 6]:
        raise PrecomposedVectorError("block trace nodes are missing, duplicated, or reordered")
    block_records: list[dict[str, Any]] = []
    for entry in block_entries:
        record = entry["record"]
        node = record["node_id"]
        manifest_placement = manifest_by_node[node]
        viewport = record.get("viewport")
        if not isinstance(viewport, dict):
            raise PrecomposedVectorError(f"block node {node} viewport is malformed")
        rect = _rect_as_list(viewport.get("rect"), f"block node {node} viewport")
        matrix = _matrix_as_list(viewport.get("matrix"), f"block node {node} matrix")
        if (
            manifest_placement.get("kind") != record.get("kind")
            or manifest_placement.get("binding_fingerprint")
            != record.get("binding_fingerprint")
            or manifest_placement.get("selected_placement_fingerprint")
            != entry.get("fingerprint")
            or manifest_placement.get("viewport") != rect
            or manifest_placement.get("matrix") != matrix
            or manifest_placement.get("scale") != viewport.get("scale")
            or manifest_placement.get("page_index") != record.get("page_index")
            or manifest_placement.get("frame_index") != record.get("frame_index")
            or manifest_placement.get("paint_ordinal")
            != viewport.get("paint_ordinal")
        ):
            raise PrecomposedVectorError(
                f"block trace/manifest placement closure failed at node {node}"
            )
        if record.get("kind") == "math_vector_block":
            baseline = record.get("math_baseline")
            metrics = manifest_placement.get("metrics")
            if (
                not isinstance(baseline, dict)
                or not isinstance(metrics, dict)
                or baseline.get("baseline") != metrics.get("baseline")
                or baseline.get("baseline_y") != rect[1] + metrics.get("baseline", -1)
                or baseline.get("pen_origin_x") + metrics.get("origin_x", 0)
                != rect[0]
            ):
                raise PrecomposedVectorError(
                    f"block math baseline closure failed at node {node}"
                )
        if (
            record.get("pagination_bounds") != record.get("paint_bounds")
            or record.get("paint_bounds") != record.get("structure_bounds")
        ):
            raise PrecomposedVectorError(
                "block pagination/paint/structure bounds diverged"
            )
        block_records.append(record)
    if not block_records[0].get("moved_to_fresh_page"):
        raise PrecomposedVectorError("page-end vector block did not move atomically")
    numbered = [
        value for value in block_records if value.get("equation_number") is not None
    ]
    if len(numbered) != 1 or [
        child.get("role") for child in numbered[0].get("structure_children", [])
    ] != ["formula", "equation_number"]:
        raise PrecomposedVectorError(
            "equation number is not independently ordered after Formula"
        )

    display_pages = display.get("pages")
    if not isinstance(display_pages, list) or len(display_pages) != 2:
        raise PrecomposedVectorError("Display page closure differs")
    display_entries: list[dict[str, Any]] = []
    for page_index, page in enumerate(display_pages):
        page_record = page.get("record") if isinstance(page, dict) else None
        if (
            not isinstance(page_record, dict)
            or page_record.get("page_index") != page_index
            or not isinstance(page_record.get("commands"), list)
        ):
            raise PrecomposedVectorError("Display page order or shape differs")
        display_entries.extend(page_record["commands"])
    if (
        display.get("command_count") != len(display_entries)
        or not all(
            isinstance(entry, dict) and isinstance(entry.get("record"), dict)
            for entry in display_entries
        )
    ):
        raise PrecomposedVectorError("Display command count or shape differs")
    display_nodes = [entry.get("record", {}).get("owner") for entry in display_entries]
    if display_nodes != [3, 4, 5, 6]:
        raise PrecomposedVectorError("Display nodes are missing, duplicated, or reordered")
    for entry in display_entries:
        command = entry["record"]
        node = command["owner"]
        manifest_placement = manifest_by_node[node]
        viewport = _rect_as_list(command.get("viewport"), f"Display node {node} viewport")
        matrix = _matrix_as_list(command.get("matrix"), f"Display node {node} matrix")
        if (
            entry.get("fingerprint")
            != manifest_placement.get("display_command_fingerprint")
            or command.get("binding_fingerprint")
            != manifest_placement.get("binding_fingerprint")
            or command.get("kind") != manifest_placement.get("kind")
            or command.get("image_id") != manifest_placement.get("image_id")
            or command.get("usage_id") != manifest_placement.get("usage_id")
            or command.get("content_key") != content_key_by_node.get(node)
            or command.get("ir_fingerprint")
            != command.get("content_key", {}).get("ir_fingerprint")
            or command.get("selected_placement_fingerprint")
            != manifest_placement.get("selected_placement_fingerprint")
            or command.get("viewport") is None
            or viewport != manifest_placement.get("viewport")
            or matrix != manifest_placement.get("matrix")
            or command.get("scale") != manifest_placement.get("scale")
            or command.get("page_index") != manifest_placement.get("page_index")
            or command.get("frame_index") != manifest_placement.get("frame_index")
            or command.get("paint_ordinal")
            != manifest_placement.get("paint_ordinal")
        ):
            raise PrecomposedVectorError(
                f"Display/manifest placement closure failed at node {node}"
            )
        metrics = manifest_placement.get("metrics")
        baseline = command.get("baseline_metrics")
        if isinstance(metrics, dict):
            if (
                not isinstance(baseline, dict)
                or baseline.get("baseline") != metrics.get("baseline")
                or baseline.get("baseline_y")
                != viewport[1] + metrics.get("baseline", -1)
                or baseline.get("pen_origin_x") + metrics.get("origin_x", 0)
                != viewport[0]
                or baseline.get("metric_receipt_fingerprint")
                != manifest_placement.get("metric_receipt_fingerprint")
            ):
                raise PrecomposedVectorError(
                    f"Display baseline closure failed at node {node}"
                )
        elif baseline is not None:
            raise PrecomposedVectorError(
                f"Display Figure unexpectedly carries baseline metrics at node {node}"
            )

    if (
        safe["placement_count"] != counts["placements"]
        or display["command_count"] != counts["placements"]
        or observation["vector_usage_count"] != counts["do"]
        or observation["form_object_count"] != counts["forms"]
        or len(math["facts"]) != counts["math_facts"]
        or len(tagged["vector_structures"]) != counts["structures"]
    ):
        raise PrecomposedVectorError("Display/manifest/PDF count closure differs")
    root_children = _verify_build_manifest_root(
        root, verification["pdf_sha256"], "build manifest root"
    )
    if (
        root_children["safe_vector"] != safe
        or root_children["math_vector"] != math
        or root_children["tagged_pdf"] != tagged
        or root_children["book_navigation"] != book
    ):
        raise PrecomposedVectorError("root build manifest does not embed exact child manifests")
    if safe["fingerprints"]["pdf_sha256"] != verification["pdf_sha256"]:
        raise PrecomposedVectorError("SafeVector manifest does not close the final PDF")
    if tagged["fingerprints"]["pdf_sha256"] != verification["pdf_sha256"]:
        raise PrecomposedVectorError("tagged manifest does not close the final PDF")
    if {fact["kind"] for fact in tagged["vector_structures"]} != {
        "inline_vector", "math_vector", "vector_figure", "math_vector_block"
    }:
        raise PrecomposedVectorError("tagged structures do not cover all four vector kinds")
    if observation["document_language"] != "ja" or not all(
        fact["language"] == "en-US" for fact in tagged["vector_structures"]
    ):
        raise PrecomposedVectorError("document/override language closure differs")
    figure_children = _verify_build_manifest_root(
        figure_root, figure["pdf_sha256"], "Figure build manifest root"
    )
    if (
        figure_children["safe_vector"].get("placement_count") != 1
        or figure_children["math_vector"].get("facts") != []
        or {
            fact.get("kind")
            for fact in figure_children["tagged_pdf"].get("vector_structures", [])
            if isinstance(fact, dict)
        }
        != {"figure"}
    ):
        raise PrecomposedVectorError("legacy Figure manifest projection differs")

    fixture_hashes = _verify_corpus(
        expectation_path.parent, expectation, repository, corpus_admission
    )
    _verify_corpus_pdf_and_display(
        directory,
        payloads,
        expected_outcome["precomposed_vector_corpus"],
        verification["corpus"],
        corpus_admission,
        expectation_path.parent,
    )
    _verify_effective_document_package(
        directory,
        payloads,
        verification,
        safe,
        math,
        tagged,
        book,
        expectation_path.parent,
        repository,
    )
    public_capabilities = (repository / "samples/machine-package/capabilities.json").read_bytes()
    public_schema = (repository / "schemas/document-package.schema.json").read_bytes()
    versioned_public_schema = (repository / "schemas/1.3/document-package.schema.json").read_bytes()
    public_cli = (repository / "workspace/crates/typaxis-cli/src/cli.rs").read_bytes()
    for forbidden in (
        b"production-book-1",
        b"inline_vector",
        b"math_vector",
        b"vector_figure",
        b"math_vector_block",
        b"svg-safe-2",
    ):
        if forbidden in public_capabilities:
            raise PrecomposedVectorError(
                f"private vocabulary leaked into capabilities: {forbidden!r}"
            )
    if (
        public_schema != versioned_public_schema
        or b"typaxis.contract/1.4" in public_schema
        or b"private-production-book" in public_cli
    ):
        raise PrecomposedVectorError("private contract or command leaked into public surfaces")

    result = {
        "artifact_records": records,
        "artifact_set_sha256": _artifact_set_digest(payloads.items()),
        "checks": [{"name": name, "result": "passed"} for name in sorted(REQUIRED_CHECKS)],
        "fixture_hashes": fixture_hashes,
    }
    if require_external_tools:
        if readiness_directory is None:
            raise PrecomposedVectorError(
                "--readiness-directory is required with --require-external-tools"
            )
        external, binary_record, tools = verify_external_evidence(
            artifact_directory=directory,
            repository=repository,
            mutool=mutool,
            pdftotext=pdftotext,
            pdfinfo=pdfinfo,
            verapdf=verapdf,
            binary=binary,
        )
        production = verify_production_readiness(
            readiness_directory,
            repository,
            vector_pdf=payloads["output.pdf"],
            vector_manifest=payloads["build-manifest-vector.json"],
        )
        if external["pdf_sha256"] != verification["pdf_sha256"]:
            raise PrecomposedVectorError("external gates are not bound to the verified PDF")
        result.update(
            {
                "binary": binary_record,
                "checks": [
                    {"name": name, "result": "passed"}
                    for name in sorted(RELEASE_REQUIRED_CHECKS)
                ],
                "external": external,
                "production": production,
                "tools": tools,
            }
        )
    elif any(
        value is not None
        for value in (readiness_directory, mutool, pdftotext, pdfinfo, verapdf, binary)
    ):
        raise PrecomposedVectorError(
            "external tool/readiness options require --require-external-tools"
        )
    return result


def _schema_validator(repository: Path) -> Draft202012Validator:
    directory = repository / "schemas/1.4"
    schemas = [
        _load_json(path, canonical=False)
        for path in sorted(directory.glob("*.schema.json"))
    ]
    selected = next(
        (
            schema
            for schema in schemas
            if schema.get("$id", "").endswith(
                "/machine-precomposed-vector-evidence.schema.json"
            )
        ),
        None,
    )
    if selected is None:
        raise PrecomposedVectorError("private evidence Schema is missing")
    try:
        for schema in schemas:
            Draft202012Validator.check_schema(schema)
        registry = Registry().with_resources(
            (schema["$id"], Resource.from_contents(schema)) for schema in schemas
        )
    except Exception as error:
        raise PrecomposedVectorError(
            f"cannot construct private Schema registry: {error}"
        ) from error
    return Draft202012Validator(selected, registry=registry)


def _validate_evidence(repository: Path, evidence: Any, label: str) -> dict[str, Any]:
    errors = sorted(
        _schema_validator(repository).iter_errors(evidence),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        detail = " | ".join(f"{error.json_path}: {error.message}" for error in errors)
        raise PrecomposedVectorError(f"{label} does not match evidence Schema: {detail}")
    assert isinstance(evidence, dict)
    names = {check["name"] for check in evidence["checks"]}
    if names != RELEASE_REQUIRED_CHECKS:
        raise PrecomposedVectorError(f"{label} does not contain the exact required checks")
    if [check["name"] for check in evidence["checks"]] != sorted(RELEASE_REQUIRED_CHECKS):
        raise PrecomposedVectorError(f"{label} check order is not canonical")
    artifact_names = [record["name"] for record in evidence["artifacts"]]
    if artifact_names != sorted(EXPECTED_ARTIFACTS):
        raise PrecomposedVectorError(f"{label} does not contain the exact artifact set")
    output_hash = next(
        record["sha256"]
        for record in evidence["artifacts"]
        if record["name"] == "output.pdf"
    )
    if evidence["external"]["pdf_sha256"] != output_hash:
        raise PrecomposedVectorError(f"{label} external gates target a different PDF")
    if evidence["external"]["tool_policy_sha256"] != _sha256(
        (_publication_root(repository) / "external-tool-policy.json").read_bytes()
    ):
        raise PrecomposedVectorError(f"{label} external tool policy is stale")
    current_production = _current_production_identity(repository)
    for member, expected in current_production.items():
        if member == "matterhorn_assessment_sha256":
            if evidence["external"]["matterhorn"]["assessment_sha256"] != expected:
                raise PrecomposedVectorError(f"{label} Matterhorn assessment is stale")
        elif evidence["production"].get(member) != expected:
            raise PrecomposedVectorError(f"{label} production identity is stale at {member}")
    host = evidence["host"]
    suffix = "apple-darwin" if host["os"] == "macos" else "unknown-linux-gnu"
    if host["target_triple"] != f"{host['arch']}-{suffix}":
        raise PrecomposedVectorError(f"{label} host target triple differs")
    return evidence


def _git_revision(repository: Path) -> str:
    try:
        completed = subprocess.run(
            ["/usr/bin/git", "-C", os.fspath(repository), "rev-parse", "--verify", "HEAD^{commit}"],
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as error:
        raise PrecomposedVectorError(f"cannot execute Git: {error}") from error
    revision = completed.stdout.decode("ascii", "strict").strip()
    if completed.returncode != 0 or not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", revision):
        raise PrecomposedVectorError("cannot resolve the source revision")
    return revision


def _source_snapshot(repository: Path, revision: str) -> str:
    try:
        completed = subprocess.run(
            [
                "/usr/bin/git",
                "-C",
                os.fspath(repository),
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
            ],
            check=False,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as error:
        raise PrecomposedVectorError(f"cannot enumerate source snapshot: {error}") from error
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise PrecomposedVectorError(f"cannot enumerate source snapshot: {detail}")
    relative_paths: list[Path] = []
    for raw in completed.stdout.split(b"\0"):
        if not raw:
            continue
        relative = Path(os.fsdecode(raw))
        if relative.is_absolute() or ".." in relative.parts:
            raise PrecomposedVectorError(f"Git returned an unsafe source path: {relative}")
        relative_paths.append(relative)
    relative_paths.sort(key=lambda path: os.fsencode(path))
    if not relative_paths:
        raise PrecomposedVectorError("source snapshot is empty")

    digest = hashlib.sha256()
    for relative in relative_paths:
        source = repository / relative
        encoded = os.fsencode(relative)
        if source.is_symlink():
            kind = b"symlink"
            payload = os.fsencode(os.readlink(source))
            executable = False
        elif source.is_file():
            kind = b"file"
            payload = source.read_bytes()
            executable = bool(source.stat().st_mode & 0o111)
        elif source.exists():
            raise PrecomposedVectorError(f"source entry is not a file: {relative}")
        else:
            # A tracked deletion is part of the selected worktree state.
            kind = b"deleted"
            payload = b""
            executable = False
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
        digest.update(kind)
        digest.update(b"x" if executable else b"-")
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    digest.update(revision.encode("ascii"))
    return digest.hexdigest()


def _source_identity(repository: Path) -> dict[str, str]:
    revision = _git_revision(repository)
    return {
        "cargo_lock_sha256": _sha256(
            (repository / "workspace/Cargo.lock").read_bytes()
        ),
        "revision": revision,
        "snapshot_sha256": _source_snapshot(repository, revision),
    }


def _verifier_identity(repository: Path) -> dict[str, str]:
    verifier_path = repository / "tools/verify_precomposed_vector.py"
    if verifier_path.is_symlink() or not verifier_path.is_file():
        raise PrecomposedVectorError("repository verifier is not a regular file")
    return {
        "id": VERIFIER_ID,
        "sha256": _sha256(verifier_path.read_bytes()),
        "version": VERIFIER_VERSION,
    }


def _host() -> dict[str, str]:
    system = platform.system().lower()
    if system == "darwin":
        operating_system = "macos"
        suffix = "apple-darwin"
    elif system == "linux":
        operating_system = "linux"
        suffix = "unknown-linux-gnu"
    else:
        raise PrecomposedVectorError(f"unsupported evidence host OS: {system}")
    architecture = platform.machine().lower().replace("arm64", "aarch64")
    if not architecture or not re.fullmatch(r"[A-Za-z0-9_.-]+", architecture):
        raise PrecomposedVectorError("host architecture is not canonical")
    return {
        "arch": architecture,
        "os": operating_system,
        "target_triple": f"{architecture}-{suffix}",
    }


def emit_host_evidence(
    output: Path,
    result: dict[str, Any],
    repository: Path,
    expectation_path: Path,
) -> dict[str, Any]:
    required_result_members = {"binary", "external", "production", "tools"}
    missing = required_result_members - set(result)
    if missing:
        raise PrecomposedVectorError(
            "host evidence requires the complete external gate: " + ", ".join(sorted(missing))
        )
    fixture = {
        **result["fixture_hashes"],
        "expected_sha256": _sha256(expectation_path.read_bytes()),
        "fixture_id": FIXTURE_ID,
    }
    evidence = {
        "artifact_set_sha256": result["artifact_set_sha256"],
        "artifacts": result["artifact_records"],
        "binary": result["binary"],
        "checks": result["checks"],
        "contract": EVIDENCE_CONTRACT,
        "external": result["external"],
        "fixture": fixture,
        "host": _host(),
        "production": result["production"],
        "result": "passed",
        "source": _source_identity(repository),
        "tools": result["tools"],
        "verifier": _verifier_identity(repository),
    }
    _validate_evidence(repository, evidence, "emitted host evidence")
    _atomic_json_write(output, evidence)
    return evidence


def _current_fixture_identity(repository: Path) -> dict[str, str]:
    expectation_path = _default_expectation(repository)
    expectation = _load_json(expectation_path)
    corpus = expectation_path.parent
    package_name = expectation.get("package")
    if not isinstance(package_name, str):
        raise PrecomposedVectorError("current fixture package path is missing")
    package_path = _contained_regular_file(corpus, package_name, "current fixture package")
    package = _load_json(package_path)
    source_rows = package.get("sources") if isinstance(package, dict) else None
    if not isinstance(source_rows, list) or not source_rows:
        raise PrecomposedVectorError("current fixture source ledger is missing")
    source_payloads: list[tuple[str, bytes]] = []
    for source_id, raw_source in enumerate(source_rows):
        source = _exact_object(
            raw_source,
            {"sha256", "source_id", "uri", "utf8_byte_length"},
            f"current fixture source {source_id}",
        )
        if source.get("source_id") != source_id or not isinstance(source.get("uri"), str):
            raise PrecomposedVectorError("current fixture source IDs differ")
        payload = _contained_regular_file(
            corpus, source["uri"], f"current fixture source {source_id}"
        ).read_bytes()
        if (
            source.get("utf8_byte_length") != len(payload)
            or source.get("sha256") != _sha256(payload)
        ):
            raise PrecomposedVectorError(
                f"current fixture source fact differs: {source_id}"
            )
        source_payloads.append((source["uri"], payload))

    def text_set(directory: str) -> str:
        root = corpus / directory
        if root.is_symlink() or not root.is_dir():
            raise PrecomposedVectorError(
                f"current fixture {directory} directory is invalid"
            )
        payloads: list[tuple[str, bytes]] = []
        for path in sorted(root.glob("*")):
            relative = path.relative_to(corpus).as_posix()
            payloads.append(
                (relative, _canonical_utf8_text_file(corpus, relative, relative))
            )
        if not payloads:
            raise PrecomposedVectorError(f"current fixture {directory} set is empty")
        return _artifact_set_digest(payloads)

    return {
        "assertion_traceability_sha256": _sha256(
            (corpus / "assertion-traceability.tsv").read_bytes()
        ),
        "cases_sha256": _sha256((corpus / "cases.tsv").read_bytes()),
        "expected_sha256": _sha256(expectation_path.read_bytes()),
        "fixture_id": FIXTURE_ID,
        "fragment_text_set_sha256": text_set("fragments"),
        "fragments_sha256": _sha256((corpus / "fragments.tsv").read_bytes()),
        "negative_sha256": _sha256((corpus / "negative-integration.tsv").read_bytes()),
        "package_sha256": _sha256(package_path.read_bytes()),
        "resources_sha256": _sha256((corpus / "resources.tsv").read_bytes()),
        "source_set_sha256": _artifact_set_digest(source_payloads),
        "tex_set_sha256": text_set("tex"),
    }


def _current_production_identity(repository: Path) -> dict[str, Any]:
    root = _publication_root(repository)
    resources = _production_resource_records(repository)
    font_uris = {
        "accessibility/job/body.ttf",
        "accessibility/job/collection.ttc",
        "cff-media/typaxis-cff-fixture.otf.hex",
        "math/job/math.ttf",
    }
    return {
        "capabilities_sha256": _sha256(
            (root / "publication-capabilities.json").read_bytes()
        ),
        "expectation_sha256": _sha256(
            (root / "publication-expectation.json").read_bytes()
        ),
        "font_resources": [row for row in resources if row["uri"] in font_uris],
        "matterhorn_assessment_sha256": _sha256(
            (root / "matterhorn-assessment.json").read_bytes()
        ),
        "resource_count": 73,
        "resource_ledger_sha256": _sha256(canonical_json_bytes(resources)),
        "resources": resources,
    }


def require_host_evidence(
    directory: Path,
    required_hosts: list[str],
    repository: Path,
) -> dict[str, Any]:
    if not required_hosts or len(set(required_hosts)) != len(required_hosts):
        raise PrecomposedVectorError("--required-host must name unique required hosts")
    if any(host not in {"macos", "linux"} for host in required_hosts):
        raise PrecomposedVectorError("required host must be macos or linux")
    entries = list(directory.iterdir())
    invalid_entries = sorted(
        entry.name
        for entry in entries
        if entry.is_symlink() or not entry.is_file() or entry.suffix != ".json"
    )
    if invalid_entries:
        raise PrecomposedVectorError(
            f"host evidence directory contains invalid entries: {invalid_entries}"
        )
    evidence_by_host: dict[str, dict[str, Any]] = {}
    for path in sorted(directory.glob("*.json")):
        evidence = _validate_evidence(repository, _load_json(path), os.fspath(path))
        host = evidence["host"]["os"]
        if host in evidence_by_host:
            raise PrecomposedVectorError(f"duplicate evidence for host {host}")
        evidence_by_host[host] = evidence
    missing = set(required_hosts) - set(evidence_by_host)
    if missing:
        raise PrecomposedVectorError(f"missing required host evidence: {sorted(missing)}")
    extra = set(evidence_by_host) - set(required_hosts)
    if extra:
        raise PrecomposedVectorError(f"unexpected host evidence: {sorted(extra)}")
    selected = [evidence_by_host[host] for host in required_hosts]
    reference = selected[0]
    for evidence in selected[1:]:
        for member in (
            "artifact_set_sha256",
            "artifacts",
            "checks",
            "fixture",
            "production",
            "source",
            "verifier",
        ):
            if evidence[member] != reference[member]:
                raise PrecomposedVectorError(f"host evidence differs at {member}")
        if evidence["binary"]["version"] != reference["binary"]["version"]:
            raise PrecomposedVectorError("host evidence differs at binary version")
        tool_versions = [(tool["name"], tool["version"]) for tool in evidence["tools"]]
        reference_tool_versions = [
            (tool["name"], tool["version"]) for tool in reference["tools"]
        ]
        if tool_versions != reference_tool_versions:
            raise PrecomposedVectorError("host evidence differs at external tool versions")
        tool_by_name = {tool["name"]: tool for tool in evidence["tools"]}
        reference_tool_by_name = {tool["name"]: tool for tool in reference["tools"]}
        if (
            tool_by_name["verapdf"]["payload_sha256"]
            != reference_tool_by_name["verapdf"]["payload_sha256"]
        ):
            raise PrecomposedVectorError("host evidence differs at pinned veraPDF payload")
        stable_external = {
            **evidence["external"],
            "differential": {
                key: value
                for key, value in evidence["external"]["differential"].items()
                if key != "render_sha256"
            },
        }
        stable_reference = {
            **reference["external"],
            "differential": {
                key: value
                for key, value in reference["external"]["differential"].items()
                if key != "render_sha256"
            },
        }
        if stable_external != stable_reference:
            raise PrecomposedVectorError("host evidence differs at stable external observations")
    if reference["source"] != _source_identity(repository):
        raise PrecomposedVectorError("host evidence contains stale source identity")
    if reference["verifier"] != _verifier_identity(repository):
        raise PrecomposedVectorError("host evidence contains stale verifier identity")
    if reference["fixture"] != _current_fixture_identity(repository):
        raise PrecomposedVectorError("host evidence contains stale fixture identity")
    return {
        "artifact_set_sha256": reference["artifact_set_sha256"],
        "contract": "typaxis.machine-precomposed-vector-host-index/2",
        "pdf_sha256": reference["external"]["pdf_sha256"],
        "production_readiness_sha256": reference["production"]["readiness_sha256"],
        "hosts": [evidence_by_host[host]["host"] for host in sorted(required_hosts)],
        "revision": reference["source"]["revision"],
    }


def _default_expectation(repository: Path) -> Path:
    return (
        repository
        / "samples/machine-package/staging/production-book-1/precomposed-vector/expected.json"
    )


def _parse_arguments(arguments: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact_directory", nargs="?", type=Path)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--expectation", type=Path)
    parser.add_argument("--emit-host-evidence", type=Path)
    parser.add_argument("--require-host-evidence", type=Path)
    parser.add_argument("--required-host", action="append", default=[])
    parser.add_argument("--prepare-readiness", type=Path)
    parser.add_argument("--readiness-directory", type=Path)
    parser.add_argument("--require-external-tools", action="store_true")
    parser.add_argument("--mutool")
    parser.add_argument("--pdftotext")
    parser.add_argument("--pdfinfo")
    parser.add_argument("--verapdf")
    parser.add_argument("--binary")
    return parser.parse_args(arguments)


def main(arguments: list[str] | None = None) -> int:
    options = _parse_arguments(sys.argv[1:] if arguments is None else arguments)
    try:
        repository = options.repository.resolve(strict=True)
        if options.prepare_readiness is not None:
            if any(
                value is not None
                for value in (
                    options.artifact_directory,
                    options.emit_host_evidence,
                    options.expectation,
                    options.require_host_evidence,
                    options.readiness_directory,
                    options.mutool,
                    options.pdftotext,
                    options.pdfinfo,
                    options.verapdf,
                    options.binary,
                )
            ) or options.required_host or options.require_external_tools:
                raise PrecomposedVectorError(
                    "readiness preparation does not accept verification options"
                )
            path = write_production_readiness_receipt(
                options.prepare_readiness, repository
            )
            result = {"receipt": os.fspath(path)}
        elif options.require_host_evidence is not None:
            if (
                options.artifact_directory is not None
                or options.emit_host_evidence is not None
                or options.expectation is not None
                or options.readiness_directory is not None
                or options.require_external_tools
                or any(
                    value is not None
                    for value in (
                        options.mutool,
                        options.pdftotext,
                        options.pdfinfo,
                        options.verapdf,
                        options.binary,
                    )
                )
            ):
                raise PrecomposedVectorError(
                    "aggregate mode does not accept build or external tool options"
                )
            result = require_host_evidence(
                options.require_host_evidence.resolve(strict=True),
                options.required_host,
                repository,
            )
        else:
            if options.artifact_directory is None:
                raise PrecomposedVectorError("artifact_directory is required in verification mode")
            if options.required_host:
                raise PrecomposedVectorError(
                    "--required-host is available only with --require-host-evidence"
                )
            expectation = options.expectation or _default_expectation(repository)
            result = verify_artifacts(
                options.artifact_directory,
                expectation,
                repository,
                require_external_tools=options.require_external_tools,
                readiness_directory=options.readiness_directory,
                mutool=options.mutool,
                pdftotext=options.pdftotext,
                pdfinfo=options.pdfinfo,
                verapdf=options.verapdf,
                binary=options.binary,
            )
            if options.emit_host_evidence is not None:
                result = emit_host_evidence(
                    options.emit_host_evidence,
                    result,
                    repository,
                    expectation,
                )
        print(canonical_json_bytes(result).decode("utf-8"))
    except (PrecomposedVectorError, OSError, UnicodeError) as error:
        print(f"precomposed-vector verification error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
