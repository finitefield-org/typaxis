#!/usr/bin/env python3
"""Verify a public machine profile or matrix and write canonical per-host evidence."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
from typing import Any, Mapping, Sequence

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

import release
import verify_pdf_differential as pdf_differential
import verify_pdf_structure as pdf_structure
import verify_reproducibility as reproducibility


EVIDENCE_CONTRACT = "typaxis.machine-profile-evidence/1"
ARTIFACT_FILES = {
    "capabilities": "capabilities.json",
    "diagnostics": "diagnostics.json",
    "manifest": "manifest.json",
    "pdf": "output.pdf",
    "trace": "trace.json",
}
ARTIFACT_SCHEMAS = {
    "capabilities": "machine-capabilities.schema.json",
    "diagnostics": "diagnostics.schema.json",
    "manifest": "build-manifest.schema.json",
    "trace": "layout-trace.schema.json",
}
REQUIRED_CHECKS = {
    "artifact_byte_identity",
    "build_package_run_1",
    "build_package_run_2",
    "capabilities_profile_closure",
    "capabilities_schema",
    "check_package",
    "clean_binary_build",
    "diagnostics_schema",
    "external_page_count",
    "external_poppler_text",
    "external_mupdf_raster",
    "manifest_schema",
    "machine_reproducibility",
    "profile_receipt_closure",
    "trace_schema",
}
PRODUCTION_REQUIRED_CHECK = "independent_pdf_structure"
REQUIRED_TOOLS = {"cargo", "mutool", "pdfinfo", "pdftotext", "python", "rustc"}
PUBLIC_PROFILES = {
    "typaxis.machine-pdf/basic-document-1",
    "typaxis.machine-pdf/columns-1",
    "typaxis.machine-pdf/float-1",
    "typaxis.machine-pdf/footnote-1",
    "typaxis.machine-pdf/header-footer-1",
    "typaxis.machine-pdf/paragraph-1",
    "typaxis.machine-pdf/production-book-1",
    "typaxis.machine-pdf/table-1",
}
ADVANCED_PROFILES = {
    "typaxis.machine-pdf/columns-1",
    "typaxis.machine-pdf/float-1",
    "typaxis.machine-pdf/header-footer-1",
}
ADVERTISED_PDF_FEATURE_MARKERS = {
    "pdf_feature:link-annotations": b"/Subtype /Link",
    "pdf_feature:named-destinations": b"/Dests",
    "pdf_feature:png-xobjects": b"/Subtype /Image",
}


class MachineProfileError(Exception):
    pass


@dataclass(frozen=True)
class FixtureResult:
    expected_path: Path
    expected: dict[str, Any]
    run_directories: tuple[Path, ...]
    artifacts: dict[str, bytes]
    differential: pdf_differential.PdfDifferentialResult
    structure: dict[str, Any] | None


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _load_json(path: Path) -> Any:
    def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise MachineProfileError(f"{path}: duplicate JSON member {key!r}")
            result[key] = value
        return result

    try:
        return json.loads(path.read_bytes(), object_pairs_hook=no_duplicates)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise MachineProfileError(f"cannot read {path}: {error}") from error


def canonical_json_bytes(value: Any) -> bytes:
    stack = [value]
    while stack:
        current = stack.pop()
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                raise MachineProfileError("canonical JSON contains an unpaired surrogate")
        elif isinstance(current, bool) or current is None:
            continue
        elif isinstance(current, int):
            if not -(2**53 - 1) <= current <= 2**53 - 1:
                raise MachineProfileError("canonical JSON integer is outside the exact range")
        elif isinstance(current, list):
            stack.extend(current)
        elif isinstance(current, dict):
            if not all(isinstance(key, str) for key in current):
                raise MachineProfileError("canonical JSON object key is not a string")
            stack.extend(current.keys())
            stack.extend(current.values())
        else:
            raise MachineProfileError(
                f"canonical JSON contains unsupported {type(current).__name__}"
            )
    # Evidence member names are contract-declared ASCII, for which Python's
    # key ordering is byte-identical to the RFC 8785 UTF-16 ordering.
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def _schema_validators(repository: Path) -> dict[str, Draft202012Validator]:
    schema_directory = repository / "schemas"
    schemas = {
        path.name: _load_json(path)
        for path in sorted(schema_directory.glob("*.schema.json"))
    }
    if "machine-profile-evidence.schema.json" not in schemas:
        raise MachineProfileError("machine profile evidence Schema is missing")
    try:
        for schema in schemas.values():
            Draft202012Validator.check_schema(schema)
        registry = Registry().with_resources(
            (schema["$id"], Resource.from_contents(schema))
            for schema in schemas.values()
        )
    except Exception as error:
        raise MachineProfileError(f"cannot construct the current Schema registry: {error}") from error
    return {
        name: Draft202012Validator(schema, registry=registry)
        for name, schema in schemas.items()
    }


def _validate_instance(
    validators: Mapping[str, Draft202012Validator],
    schema_name: str,
    instance: Any,
    label: str,
) -> None:
    errors = sorted(
        validators[schema_name].iter_errors(instance),
        key=lambda error: list(error.absolute_path),
    )
    if errors:
        details = " | ".join(f"{error.json_path}: {error.message}" for error in errors)
        raise MachineProfileError(f"{label} is not valid against {schema_name}: {details}")


def _run_capture(
    command: Sequence[os.PathLike[str] | str],
    *,
    cwd: Path,
    environment: Mapping[str, str],
) -> bytes:
    try:
        return reproducibility._run_capture(command, cwd=cwd, environment=environment)
    except reproducibility.ReproducibilityError as error:
        raise MachineProfileError(str(error)) from error


def _run_checked(
    command: Sequence[os.PathLike[str] | str],
    *,
    cwd: Path,
    environment: Mapping[str, str],
) -> None:
    try:
        reproducibility._run_checked(command, cwd=cwd, environment=environment)
    except reproducibility.ReproducibilityError as error:
        raise MachineProfileError(str(error)) from error


def _fixture_paths(repository: Path, fixture: Path) -> list[Path]:
    candidate = fixture if fixture.is_absolute() else repository / fixture
    candidate = candidate.resolve(strict=True)
    try:
        candidate.relative_to(repository)
    except ValueError as error:
        raise MachineProfileError("fixture must be inside the repository") from error
    instance = _load_json(candidate)
    if not isinstance(instance, dict):
        raise MachineProfileError("fixture root must be an object")
    if instance.get("contract") == "typaxis.machine-fixture-matrix/1":
        records = instance.get("fixtures")
        if not isinstance(records, list):
            raise MachineProfileError("fixture matrix has no fixtures array")
        machine_root = repository / "samples/machine-package"
        paths: list[Path] = []
        for record in records:
            if not isinstance(record, dict) or not isinstance(record.get("expected"), str):
                raise MachineProfileError("fixture matrix contains an invalid record")
            expected_path = (machine_root / record["expected"]).resolve(strict=True)
            expected = _load_json(expected_path)
            if isinstance(expected, dict) and expected.get("fixture_class") == "positive":
                paths.append(expected_path)
        if not paths:
            raise MachineProfileError("fixture matrix contains no positive public fixture")
        return paths
    if instance.get("command") != "build-package":
        raise MachineProfileError("--fixture must name expected.json or a machine matrix")
    return [candidate]


def _check_arguments(expected: dict[str, Any], diagnostics: Path) -> list[str]:
    raw = expected.get("arguments")
    if not isinstance(raw, list) or not raw or not all(isinstance(item, str) for item in raw):
        raise MachineProfileError("fixture arguments are invalid")
    arguments = [item for item in raw]
    output = ["check-package", arguments[0]]
    index = 1
    value_options_to_drop = {
        "-o",
        "--output",
        "--trace",
        "--emit-build-manifest",
        "--emit-diagnostics",
    }
    flags_to_drop = {"--force", "--no-compress", "--strict", "--trace-text"}
    while index < len(arguments):
        option = arguments[index]
        if option in value_options_to_drop:
            if index + 1 >= len(arguments):
                raise MachineProfileError(f"fixture option lacks a value: {option}")
            index += 2
            continue
        if option in flags_to_drop:
            index += 1
            continue
        output.append(option)
        if option in {"--config", "--package-root", "--profile", "--resource-root"} or (
            option.startswith("--max-") and "=" not in option
        ):
            if index + 1 >= len(arguments):
                raise MachineProfileError(f"fixture option lacks a value: {option}")
            output.append(arguments[index + 1])
            index += 2
        else:
            index += 1
    output.extend(["--emit-diagnostics", os.fspath(diagnostics)])
    return output


def _capabilities(
    executable: Path,
    scratch: Path,
    environment: Mapping[str, str],
) -> bytes:
    scratch.mkdir()
    (scratch / "typaxis.toml").write_bytes(b"not valid TOML\0")
    hostile = dict(environment)
    hostile.update(
        {
            "LC_ALL": "typaxis-invalid-locale",
            "TYPAXIS_LIMITS__MAX_PAGES": "not-an-integer",
            "TYPAXIS_UNKNOWN": "must-not-be-read",
        }
    )
    return _run_capture(
        [executable, "capabilities", "--format", "json"],
        cwd=scratch,
        environment=hostile,
    )


def _assert_profile_closure(
    capabilities: dict[str, Any], requested_profiles: set[str]
) -> None:
    machine = capabilities.get("machine_input")
    profiles = machine.get("profiles") if isinstance(machine, dict) else None
    if not isinstance(machine, dict) or not isinstance(profiles, list):
        raise MachineProfileError("capabilities have no public machine profile list")
    profile_ids = {
        profile.get("id") for profile in profiles if isinstance(profile, dict)
    }
    if profile_ids != PUBLIC_PROFILES or len(profiles) != len(PUBLIC_PROFILES):
        raise MachineProfileError("capabilities do not advertise the exact public profile set")
    if machine.get("default_profile") != "typaxis.machine-pdf/paragraph-1":
        raise MachineProfileError("capabilities changed the compatibility default profile")
    if machine.get("document_package_contracts") != [
        "typaxis.contract/1.0",
        "typaxis.contract/1.1",
        "typaxis.contract/1.2",
        "typaxis.contract/1.3",
        "typaxis.contract/1.4",
    ]:
        raise MachineProfileError("capabilities changed the accepted contract migration set")
    if capabilities.get("contract") != "typaxis.contract/1.4":
        raise MachineProfileError("capabilities are not published under the current contract")
    if not requested_profiles or not requested_profiles <= profile_ids:
        raise MachineProfileError("fixture requests a profile absent from capabilities")


def _assert_profile_receipt_closure(
    expected: dict[str, Any], manifest: dict[str, Any], trace: dict[str, Any]
) -> None:
    profile = expected.get("profile")
    package_input = manifest.get("package_input")
    layout = manifest.get("layout")
    if (
        profile not in PUBLIC_PROFILES
        or manifest.get("input_profile") != profile
        or not isinstance(package_input, dict)
        or not isinstance(layout, dict)
    ):
        raise MachineProfileError("manifest does not bind the resolved public profile")
    receipt = package_input.get("profile_receipt_sha256")
    if (
        not isinstance(receipt, str)
        or len(receipt) != 64
        or layout.get("profile_receipt_sha256") != receipt
        or trace.get("profile_receipt_sha256") != receipt
    ):
        raise MachineProfileError("profile receipt differs across package, trace, and manifest")
    manifest_flow = layout.get("flow_registry_sha256")
    trace_flow = trace.get("flow_registry_sha256")
    if manifest_flow != trace_flow:
        raise MachineProfileError("flow registry differs between trace and manifest")
    if profile in {
        "typaxis.machine-pdf/basic-document-1",
        "typaxis.machine-pdf/columns-1",
        "typaxis.machine-pdf/float-1",
        "typaxis.machine-pdf/footnote-1",
        "typaxis.machine-pdf/header-footer-1",
        "typaxis.machine-pdf/production-book-1",
        "typaxis.machine-pdf/table-1",
    }:
        if not isinstance(manifest_flow, str) or len(manifest_flow) != 64:
            raise MachineProfileError(f"{profile} lacks its selected flow registry binding")
    elif manifest_flow is not None:
        raise MachineProfileError("paragraph-1 unexpectedly carries a basic flow registry")
    manifest_tables = manifest.get("table_layouts")
    trace_tables = trace.get("table_layouts")
    if manifest_tables != trace_tables:
        raise MachineProfileError("table selected state differs between trace and manifest")
    if profile == "typaxis.machine-pdf/table-1":
        if not isinstance(manifest_tables, list):
            raise MachineProfileError("table-1 lacks selected table layout facts")
    elif manifest_tables is not None or trace_tables is not None:
        raise MachineProfileError("an older profile unexpectedly carries table layout facts")
    manifest_footnotes = manifest.get("footnote_layout")
    trace_footnotes = trace.get("footnote_layout")
    if manifest_footnotes != trace_footnotes:
        raise MachineProfileError("footnote selected state differs between trace and manifest")
    if profile == "typaxis.machine-pdf/footnote-1":
        if not isinstance(manifest_footnotes, dict):
            raise MachineProfileError("footnote-1 lacks selected footnote layout facts")
    elif manifest_footnotes is not None or trace_footnotes is not None:
        raise MachineProfileError("an older profile unexpectedly carries footnote layout facts")
    manifest_advanced = manifest.get("advanced_pagination")
    trace_advanced = trace.get("advanced_pagination")
    if manifest_advanced != trace_advanced:
        raise MachineProfileError("advanced selected state differs between trace and manifest")
    if profile in ADVANCED_PROFILES:
        if not isinstance(manifest_advanced, dict):
            raise MachineProfileError("advanced profile lacks selected pagination facts")
        if (
            manifest_advanced.get("algorithm")
            != "typaxis.advanced-pagination-manifest/1"
            or manifest_advanced.get("profile") != profile
            or manifest_advanced.get("profile_receipt_sha256") != receipt
            or manifest_advanced.get("flow_registry_sha256") != manifest_flow
            or manifest_advanced.get("selected_layout_sha256")
            != layout.get("final_fingerprint")
            or not isinstance(manifest_advanced.get("paint_closure_sha256"), str)
            or len(manifest_advanced["paint_closure_sha256"]) != 64
        ):
            raise MachineProfileError("advanced receipt graph is incomplete or inconsistent")
        pages = manifest_advanced.get("pages")
        output = manifest.get("output")
        if (
            not isinstance(pages, list)
            or not pages
            or not isinstance(output, dict)
            or output.get("page_count") != len(pages)
        ):
            raise MachineProfileError("advanced page closure differs from the PDF output")
        previous_queue: list[Any] = []
        for page_index, page in enumerate(pages):
            frames = page.get("frames") if isinstance(page, dict) else None
            if (
                not isinstance(page, dict)
                or page.get("page_index") != page_index
                or not isinstance(frames, list)
                or not frames
                or page.get("float_queue_before") != previous_queue
            ):
                raise MachineProfileError("advanced page/frame/queue order is incomplete")
            for frame in frames:
                before = frame.get("before_position") if isinstance(frame, dict) else None
                after = frame.get("after_position") if isinstance(frame, dict) else None
                if (
                    not isinstance(before, dict)
                    or not isinstance(after, dict)
                    or before.get("flow_id") != after.get("flow_id")
                    or type(before.get("ordinal")) is not int
                    or type(after.get("ordinal")) is not int
                    or after["ordinal"] < before["ordinal"]
                ):
                    raise MachineProfileError("advanced frame cursor regressed or changed flow")
            next_queue = page.get("float_queue_after")
            if not isinstance(next_queue, list):
                raise MachineProfileError("advanced page lacks its outgoing float queue")
            previous_queue = next_queue
        if previous_queue:
            raise MachineProfileError("advanced selected state ends with a nonterminal float queue")
        if profile == "typaxis.machine-pdf/columns-1":
            balances = [page.get("balance") for page in pages]
            if (
                not isinstance(balances[-1], dict)
                or balances[-1].get("algorithm")
                != "typaxis.column-balance-candidates/1"
                or any(balance is not None for balance in balances[:-1])
            ):
                raise MachineProfileError("columns final-page balance closure is incomplete")
        elif profile == "typaxis.machine-pdf/header-footer-1":
            if any(
                page.get("balance") is not None
                or page.get("float_queue_before")
                or page.get("float_placements")
                or page.get("float_carries")
                or page.get("float_queue_after")
                for page in pages
            ):
                raise MachineProfileError("header/footer profile carries forbidden column/float state")
    elif manifest_advanced is not None or trace_advanced is not None:
        raise MachineProfileError("an older profile unexpectedly carries advanced pagination facts")


def _assert_production_vector_closure(
    expected: dict[str, Any], manifest: dict[str, Any], trace: dict[str, Any]
) -> None:
    if expected.get("profile") != "typaxis.machine-pdf/production-book-1":
        return
    if (
        manifest.get("contract") != "typaxis.contract/1.4"
        or trace.get("contract") != "typaxis.contract/1.4"
        or trace.get("coordinate_unit") != "pdf_point_1_65536"
        or manifest.get("status") != "built"
    ):
        raise MachineProfileError("production artifacts do not use the public 1.4 contract")
    pairs = (
        (
            "book_navigation_manifest",
            "book_navigation_manifest_fingerprint",
            "typaxis.book-navigation-manifest/2",
        ),
        (
            "math_vector_manifest",
            "math_vector_manifest_fingerprint",
            "typaxis.math-vector-manifest/1",
        ),
        (
            "safe_vector_manifest",
            "safe_vector_manifest_fingerprint",
            "typaxis.safe-vector-manifest/2",
        ),
        (
            "tagged_pdf_manifest",
            "tagged_pdf_manifest_fingerprint",
            "typaxis.tagged-pdf-manifest/2",
        ),
    )
    children: dict[str, dict[str, Any]] = {}
    for member, fingerprint_member, algorithm in pairs:
        child = manifest.get(member)
        if (
            not isinstance(child, dict)
            or child.get("algorithm") != algorithm
            or child.get("contract") != "typaxis.contract/1.4"
            or manifest.get(fingerprint_member)
            != _sha256(canonical_json_bytes(child))
        ):
            raise MachineProfileError(f"production child manifest is not closed: {member}")
        children[member] = child
    layout = manifest.get("layout")
    if (
        not isinstance(layout, dict)
        or layout.get("final_fingerprint") != trace.get("selected_layout_sha256")
        or layout.get("flow_registry_sha256") != trace.get("flow_registry_sha256")
        or not isinstance(trace.get("block_layout_sha256"), str)
        or not isinstance(trace.get("inline_layout_sha256"), str)
        or not isinstance(trace.get("vector_display_sha256"), str)
        or not isinstance(trace.get("fragment_count"), int)
        or trace["fragment_count"] <= 0
    ):
        raise MachineProfileError("production selected-layout trace closure differs")
    output = manifest.get("output")
    output_sha256 = output.get("sha256") if isinstance(output, dict) else None
    pdf_hashes = {
        children["book_navigation_manifest"].get("fingerprints", {}).get("pdf_sha256"),
        children["safe_vector_manifest"].get("fingerprints", {}).get("pdf_sha256"),
        children["tagged_pdf_manifest"].get("fingerprints", {}).get("pdf_sha256"),
        output_sha256,
    }
    if None in pdf_hashes or len(pdf_hashes) != 1:
        raise MachineProfileError("production child manifests do not bind one final PDF")
    images = manifest.get("images")
    fonts = manifest.get("fonts")
    if not isinstance(images, list) or not images or not isinstance(fonts, list) or not fonts:
        raise MachineProfileError("production root manifest lacks admitted media facts")
    for record in [*images, *fonts]:
        declaration = record.get("media_declaration") if isinstance(record, dict) else None
        if (
            not isinstance(declaration, dict)
            or declaration.get("kind") != "declared"
            or declaration.get("media_type") != record.get("attested_media_kind")
        ):
            raise MachineProfileError("production declared and attested media differ")
    images_by_id = {record.get("image_id"): record for record in images}
    safe = children["safe_vector_manifest"]
    resources = safe.get("resources")
    if not isinstance(resources, list) or not resources:
        raise MachineProfileError("production SafeVector manifest has no vector resource")
    for resource in resources:
        content_key = resource.get("content_key") if isinstance(resource, dict) else None
        aliases = resource.get("aliases") if isinstance(resource, dict) else None
        if not isinstance(content_key, dict) or not isinstance(aliases, list) or not aliases:
            raise MachineProfileError("production SafeVector alias closure is incomplete")
        for alias in aliases:
            root_record = images_by_id.get(alias.get("image_id"))
            if (
                not isinstance(root_record, dict)
                or root_record.get("uri") != alias.get("uri")
                or root_record.get("sha256") != alias.get("admitted_sha256")
                or root_record.get("attested_media_kind") != content_key.get("media_type")
            ):
                raise MachineProfileError("production vector alias differs from root media")
    math_facts = children["math_vector_manifest"].get("facts")
    if not isinstance(math_facts, list) or not math_facts:
        raise MachineProfileError("production math-vector facts are empty")
    for fact in math_facts:
        if (
            not isinstance(fact.get("source_tex"), dict)
            or not isinstance(fact.get("producer"), dict)
            or not isinstance(fact.get("owner_source_span"), dict)
            or not isinstance(fact.get("resolved_actual_text_sha256"), str)
        ):
            raise MachineProfileError("production math source/accessibility receipt is incomplete")


def _assert_fixture_resource_hashes(expected_path: Path, expected: dict[str, Any]) -> None:
    root = expected_path.parent / "job"
    records = expected.get("resource_hashes")
    if not isinstance(records, list):
        raise MachineProfileError("fixture resource ledger is not an array")
    for record in records:
        uri = record.get("uri") if isinstance(record, dict) else None
        if not isinstance(uri, str):
            raise MachineProfileError("fixture resource ledger contains an invalid URI")
        path = (root / uri).resolve(strict=True)
        try:
            path.relative_to(root.resolve(strict=True))
        except ValueError as error:
            raise MachineProfileError("fixture resource ledger escapes its job root") from error
        payload = path.read_bytes()
        if len(payload) != record.get("bytes") or _sha256(payload) != record.get("sha256"):
            raise MachineProfileError(f"fixture resource hash differs: {uri}")


def _assert_production_handoff(
    repository: Path, expected: dict[str, Any], expected_capabilities: bytes
) -> None:
    if expected.get("profile") != "typaxis.machine-pdf/production-book-1":
        return
    root = repository / "samples/machine-package/staging/production-book-1"
    sealed_path = root / "publication-expectation.json"
    sealed = _load_json(sealed_path)
    if (
        expected.get("advertised_item_coverage")
        != sealed.get("advertised_item_coverage")
        or (root / "publication-capabilities.json").read_bytes()
        != expected_capabilities + b"\n"
    ):
        raise MachineProfileError("public production fixture differs from the sealed V19 handoff")
    records = sealed.get("resource_hashes")
    if not isinstance(records, list) or len(records) != 73:
        raise MachineProfileError("sealed V19 resource ledger is incomplete")
    for record in records:
        uri = record.get("uri") if isinstance(record, dict) else None
        if not isinstance(uri, str):
            raise MachineProfileError("sealed V19 resource ledger contains an invalid URI")
        path = (root / uri).resolve(strict=True)
        try:
            path.relative_to(root.resolve(strict=True))
        except ValueError as error:
            raise MachineProfileError("sealed V19 resource ledger escapes its root") from error
        payload = path.read_bytes()
        if len(payload) != record.get("bytes") or _sha256(payload) != record.get("sha256"):
            raise MachineProfileError(f"sealed V19 resource hash differs: {uri}")


def _assert_footnote_layout_facts(
    expected: dict[str, Any], manifest: dict[str, Any], trace: dict[str, Any]
) -> None:
    if expected.get("profile") != "typaxis.machine-pdf/footnote-1":
        return
    facts = manifest.get("footnote_layout")
    layout = manifest.get("layout")
    output = manifest.get("output")
    pages = facts.get("pages") if isinstance(facts, dict) else None
    page_count = output.get("page_count") if isinstance(output, dict) else None
    if (
        not isinstance(facts, dict)
        or facts.get("algorithm") != "typaxis.footnote-manifest/1"
        or not isinstance(layout, dict)
        or not isinstance(pages, list)
        or not pages
        or type(page_count) is not int
        or len(pages) != page_count
        or facts.get("body_layout_sha256") != layout.get("final_fingerprint")
    ):
        raise MachineProfileError("footnote body/page selected-state closure is incomplete")
    for key in (
        "body_layout_sha256",
        "paint_sha256",
        "profile_sha256",
        "registry_sha256",
        "selected_layout_sha256",
    ):
        if not isinstance(facts.get(key), str) or len(facts[key]) != 64:
            raise MachineProfileError(f"footnote layout lacks a canonical {key} binding")

    selected_state = trace.get("result", {}).get("selected_state")
    trace_passes = trace.get("passes")
    if (
        type(selected_state) is not int
        or not isinstance(trace_passes, list)
        or selected_state < 1
        or selected_state > len(trace_passes)
    ):
        raise MachineProfileError("footnote trace lacks its selected body pass")
    selected_pages = trace_passes[selected_state - 1].get("state", {}).get("pages")
    if not isinstance(selected_pages, list) or len(selected_pages) != len(pages):
        raise MachineProfileError("footnote trace/body page counts differ")

    next_assignment = 0
    flow_state: dict[int, tuple[str, int, int, int, bool]] = {}
    footnote_flows: dict[str, int] = {}
    assignment_flows: dict[int, int] = {}
    previous_body_position = -1
    previous_body_terminal = False
    for page_index, page in enumerate(pages):
        selected_page = selected_pages[page_index]
        if not isinstance(page, dict) or not isinstance(selected_page, dict):
            raise MachineProfileError("footnote page fact is not an object")
        ordered = page.get("ordered_footnote_ids")
        flows = page.get("flows")
        reservation = page.get("reservation")
        body_position = page.get("body_continuation_position")
        body_terminal = page.get("body_continuation_terminal")
        if (
            page.get("page_index") != page_index
            or type(body_position) is not int
            or body_position < 0
            or type(body_terminal) is not bool
            or body_position < previous_body_position
            or (previous_body_terminal and body_position != previous_body_position)
            or not isinstance(page.get("body_fingerprint"), str)
            or len(page["body_fingerprint"]) != 64
            or type(page.get("evaluation_count")) is not int
            or page["evaluation_count"] < 2
            or not isinstance(ordered, list)
            or not all(isinstance(item, str) for item in ordered)
            or len(ordered) != len(set(ordered))
            or not isinstance(flows, list)
            or len(flows) != len(ordered)
            or type(reservation) is not int
            or reservation < 0
            or (reservation == 0) != (not flows)
            or selected_page.get("page_index") != page_index
        ):
            raise MachineProfileError("footnote page order/reservation facts are invalid")
        selected_ids = selected_page.get("footnote_ids")
        if (
            not isinstance(selected_ids, list)
            or selected_ids
            != sorted(selected_ids, key=lambda item: item.encode("utf-8"))
            or len(selected_ids) != len(set(selected_ids))
            or selected_ids
            != sorted(ordered, key=lambda item: item.encode("utf-8"))
        ):
            raise MachineProfileError("footnote trace ID projection is not canonical")
        frames = selected_page.get("frames")
        body_entry = (
            frames[0]
            if isinstance(frames, list) and frames and isinstance(frames[0], dict)
            else None
        )
        body_frame = body_entry.get("bounds") if isinstance(body_entry, dict) else None
        if (
            not isinstance(frames, list)
            or len(frames) != (1 if reservation == 0 else 2)
            or not isinstance(body_entry, dict)
            or body_entry.get("kind") != "body"
            or not isinstance(body_frame, dict)
        ):
            raise MachineProfileError("footnote trace frame set contradicts reservation")
        if reservation > 0:
            footnote_entry = frames[1]
            footnote_frame = (
                footnote_entry.get("bounds")
                if isinstance(footnote_entry, dict)
                else None
            )
            if (
                not isinstance(footnote_entry, dict)
                or footnote_entry.get("kind") != "footnote"
                or not isinstance(footnote_frame, dict)
                or footnote_frame.get("height") != reservation
                or footnote_frame.get("x") != body_frame.get("x")
                or footnote_frame.get("width") != body_frame.get("width")
                or type(footnote_frame.get("y")) is not int
                or type(body_frame.get("y")) is not int
                or type(body_frame.get("height")) is not int
                or footnote_frame["y"] + reservation
                != body_frame["y"] + body_frame["height"]
            ):
                raise MachineProfileError("footnote trace frame geometry is not exact")
        previous_assignment = -1
        for footnote_id, flow in zip(ordered, flows):
            if not isinstance(flow, dict) or flow.get("footnote_id") != footnote_id:
                raise MachineProfileError("footnote flow/ID paint order differs")
            flow_id = flow.get("flow_id")
            assignment = flow.get("assignment_ordinal")
            before = flow.get("before_fragment")
            after = flow.get("after_fragment")
            incoming = flow.get("incoming_source_page")
            carries = flow.get("carries_out")
            if (
                type(flow_id) is not int
                or flow_id < 0
                or type(assignment) is not int
                or assignment <= previous_assignment
                or type(before) is not int
                or type(after) is not int
                or before < 0
                or after <= before
                or type(carries) is not bool
                or (incoming is not None and type(incoming) is not int)
            ):
                raise MachineProfileError("footnote cursor/assignment fact is invalid")
            previous_assignment = assignment
            prior = flow_state.get(flow_id)
            if prior is None:
                if (
                    incoming is not None
                    or before != 0
                    or assignment != next_assignment
                    or footnote_id in footnote_flows
                    or assignment in assignment_flows
                ):
                    raise MachineProfileError("new footnote assignment is not dense or initial")
                footnote_flows[footnote_id] = flow_id
                assignment_flows[assignment] = flow_id
                next_assignment += 1
            else:
                prior_id, prior_assignment, prior_after, prior_page, prior_carries = prior
                if (
                    footnote_id != prior_id
                    or assignment != prior_assignment
                    or not prior_carries
                    or incoming != prior_page
                    or page_index != prior_page + 1
                    or before != prior_after
                ):
                    raise MachineProfileError("footnote carry edge is missing, stale, or reordered")
            flow_state[flow_id] = (footnote_id, assignment, after, page_index, carries)
        if [flow.get("assignment_ordinal") for flow in flows] != sorted(
            flow.get("assignment_ordinal") for flow in flows
        ):
            raise MachineProfileError("footnote page assignment order is not canonical")
        previous_body_position = body_position
        previous_body_terminal = body_terminal
    if not previous_body_terminal:
        raise MachineProfileError("footnote selected state ends before the body terminal")
    if any(state[4] for state in flow_state.values()):
        raise MachineProfileError("footnote selected state ends with an unresolved carry")
    if set(flow_state) != set(range(len(flow_state))):
        raise MachineProfileError("footnote flow IDs are not dense")


def _assert_table_layout_facts(expected: dict[str, Any], manifest: dict[str, Any]) -> None:
    if expected.get("profile") != "typaxis.machine-pdf/table-1":
        return
    tables = manifest.get("table_layouts")
    output = manifest.get("output")
    output_pages = output.get("page_count") if isinstance(output, dict) else None
    if not isinstance(tables, list) or not isinstance(output_pages, int):
        raise MachineProfileError("table fixture lacks selected table/page facts")
    if not tables:
        return
    previous_owner = -1
    for table in tables:
        if not isinstance(table, dict):
            raise MachineProfileError("table layout fact is not an object")
        owner = table.get("table_node_id")
        start = table.get("target_page_start")
        page_count = table.get("page_count")
        if (
            not isinstance(owner, int)
            or owner <= previous_owner
            or not isinstance(start, int)
            or not isinstance(page_count, int)
            or page_count <= 0
            or start < 0
            or start + page_count > output_pages
        ):
            raise MachineProfileError("table page range or canonical owner order is invalid")
        previous_owner = owner
        columns = table.get("columns")
        residual = table.get("rounding_residual")
        recipient = table.get("residual_recipient")
        if (
            not isinstance(columns, list)
            or not columns
            or type(residual) is not int
            or (recipient is not None and type(recipient) is not int)
        ):
            raise MachineProfileError("table column/residual facts are incomplete")
        fraction_ordinals: list[int] = []
        for ordinal, column in enumerate(columns):
            if not isinstance(column, dict) or column.get("column_ordinal") != ordinal:
                raise MachineProfileError("table columns are not in dense canonical order")
            kind = column.get("input_kind")
            input_value = column.get("input_value")
            rounded = column.get("rounded_fraction_width")
            final_width = column.get("final_width")
            if (
                type(input_value) is not int
                or input_value <= 0
                or type(final_width) is not int
                or final_width <= 0
            ):
                raise MachineProfileError("table column width fact is invalid")
            if kind == "fixed":
                if rounded is not None or final_width != input_value:
                    raise MachineProfileError("fixed table column was reinterpreted")
            elif kind == "fraction":
                if type(rounded) is not int or rounded < 0:
                    raise MachineProfileError("fraction table column lacks its rounded share")
                fraction_ordinals.append(ordinal)
            else:
                raise MachineProfileError("table column input kind is not closed")
        if fraction_ordinals:
            if recipient != fraction_ordinals[-1]:
                raise MachineProfileError("table residual recipient is not the last fraction")
            for ordinal in fraction_ordinals:
                column = columns[ordinal]
                expected_width = column["rounded_fraction_width"]
                if ordinal == recipient:
                    expected_width += residual
                if column["final_width"] != expected_width:
                    raise MachineProfileError("fraction table final width differs from its receipt")
        elif recipient is not None or residual != 0:
            raise MachineProfileError("fixed-only table unexpectedly carries a residual")

        rows = table.get("rows")
        sources = table.get("header_sources")
        occurrences = table.get("header_occurrences")
        if not all(isinstance(value, list) for value in (rows, sources, occurrences)):
            raise MachineProfileError("table row/header closure is incomplete")
        if any(
            not isinstance(row, dict)
            or not start <= row.get("page_index", -1) < start + page_count
            for row in rows
        ):
            raise MachineProfileError("table body row targets a wrong PDF page")
        for row in rows:
            _assert_table_manifest_cells(row.get("cells"), len(columns))
            for key in ("continuation_before", "continuation_after"):
                _assert_table_manifest_continuation(row.get(key), len(columns))
        for source in sources:
            if not isinstance(source, dict):
                raise MachineProfileError("table header source fact is not an object")
            _assert_table_manifest_cells(source.get("cells"), len(columns))
        if not sources:
            if occurrences:
                raise MachineProfileError("header occurrence exists without a selected source")
            continue
        source_keys = [
            (source.get("source_fragment_id"), source.get("row_node_id"))
            for source in sources
            if isinstance(source, dict)
        ]
        if len(source_keys) != len(sources) or len(occurrences) % len(sources) != 0:
            raise MachineProfileError("table header source/occurrence cardinality differs")
        repetition_count = len(occurrences) // len(sources)
        if repetition_count != page_count:
            raise MachineProfileError("table header was not repeated on every selected page")
        for repetition_index in range(repetition_count):
            group = occurrences[
                repetition_index * len(sources) : (repetition_index + 1) * len(sources)
            ]
            observed_keys = [
                (item.get("source_fragment_id"), item.get("row_node_id"))
                for item in group
                if isinstance(item, dict)
            ]
            if (
                observed_keys != source_keys
                or any(item.get("repetition_index") != repetition_index for item in group)
                or any(item.get("target_page_index") != start + repetition_index for item in group)
            ):
                raise MachineProfileError("table header repetition is missing, stale, or reordered")


def _assert_table_manifest_cells(cells: Any, column_count: int) -> None:
    if not isinstance(cells, list) or not cells:
        raise MachineProfileError("table row/header lacks selected cell facts")
    previous_column = -1
    for cell in cells:
        if not isinstance(cell, dict):
            raise MachineProfileError("table selected cell fact is not an object")
        column = cell.get("column_ordinal")
        colspan = cell.get("colspan")
        rowspan = cell.get("rowspan")
        if (
            type(column) is not int
            or column <= previous_column
            or type(colspan) is not int
            or colspan <= 0
            or column + colspan > column_count
            or type(rowspan) is not int
            or rowspan <= 0
        ):
            raise MachineProfileError("table selected cell span/order is invalid")
        previous_column = column
        integer_fields = (
            "after_fragment_ordinal",
            "before_fragment_ordinal",
            "cell_node_id",
            "flow_id",
            "selected_block_extent",
            "vertical_offset_after",
            "vertical_offset_before",
        )
        if any(type(cell.get(key)) is not int or cell[key] < 0 for key in integer_fields):
            raise MachineProfileError("table selected cell cursor/geometry is incomplete")
        if any(type(cell.get(key)) is not bool for key in ("after_terminal", "before_terminal")):
            raise MachineProfileError("table selected cell terminal state is incomplete")


def _assert_table_manifest_continuation(continuation: Any, column_count: int) -> None:
    if not isinstance(continuation, dict) or not isinstance(continuation.get("entries"), list):
        raise MachineProfileError("table rowspan continuation is incomplete")
    if type(continuation.get("logical_row_ordinal")) is not int:
        raise MachineProfileError("table rowspan continuation lacks its logical row")
    previous_column = -1
    for entry in continuation["entries"]:
        cursor = entry.get("cell_flow_cursor") if isinstance(entry, dict) else None
        column = entry.get("column_ordinal") if isinstance(entry, dict) else None
        if (
            not isinstance(entry, dict)
            or not isinstance(cursor, dict)
            or type(column) is not int
            or column <= previous_column
            or column >= column_count
            or type(entry.get("remaining_logical_rows")) is not int
            or entry["remaining_logical_rows"] <= 0
            or type(entry.get("vertical_offset")) is not int
            or entry["vertical_offset"] < 0
            or type(entry.get("cell_node_id")) is not int
            or type(entry.get("flow_id")) is not int
            or cursor.get("flow_id") != entry.get("flow_id")
            or type(cursor.get("next_fragment_ordinal")) is not int
            or type(cursor.get("terminal")) is not bool
        ):
            raise MachineProfileError("table rowspan continuation entry is invalid")
        previous_column = column


def _assert_table_zero_decoration(
    expected: dict[str, Any],
    pdf: Path,
    *,
    repository: Path,
    environment: Mapping[str, str],
    mutool: str,
) -> None:
    if expected.get("profile") != "typaxis.machine-pdf/table-1":
        return
    trace = _run_capture(
        [mutool, "draw", "-F", "trace", pdf],
        cwd=repository,
        environment=environment,
    )
    forbidden = (b"<fill_path", b"<stroke_path", b"<clip_path")
    if any(token in trace for token in forbidden):
        raise MachineProfileError("table PDF contains an unexpected path decoration operation")
    expected_pages = expected.get("expected", {}).get("page_count")
    if isinstance(expected_pages, int) and trace.count(b"<page ") != expected_pages:
        raise MachineProfileError("MuPDF trace page count differs from the table expectation")


def _assert_footnote_separator_paint(
    expected: dict[str, Any],
    manifest: dict[str, Any],
    pdf: Path,
    *,
    repository: Path,
    environment: Mapping[str, str],
    mutool: str,
) -> None:
    if expected.get("profile") != "typaxis.machine-pdf/footnote-1":
        return
    facts = manifest.get("footnote_layout")
    pages = facts.get("pages") if isinstance(facts, dict) else None
    if not isinstance(pages, list):
        raise MachineProfileError("footnote separator check lacks selected page facts")
    expected_separators = sum(
        1
        for page in pages
        if isinstance(page, dict) and isinstance(page.get("flows"), list) and page["flows"]
    )
    trace = _run_capture(
        [mutool, "draw", "-F", "trace", pdf],
        cwd=repository,
        environment=environment,
    )
    if (
        trace.count(b"<stroke_path") != expected_separators
        or b"<fill_path" in trace
        or b"<clip_path" in trace
    ):
        raise MachineProfileError("footnote PDF separator paint count/policy differs")


def _assert_advertised_pdf_features(expected: dict[str, Any], pdf: Path) -> None:
    coverage = expected.get("advertised_item_coverage")
    if not isinstance(coverage, list) or not all(isinstance(item, str) for item in coverage):
        raise MachineProfileError("positive fixture lacks advertised item coverage")
    payload = pdf.read_bytes()
    if expected.get("profile") == "typaxis.machine-pdf/production-book-1":
        for feature, marker in {
            "pdf_feature:outlines": b"/Outlines",
            "pdf_feature:png-xobjects": b"/Subtype /Image",
            "pdf_feature:tagged-pdf": b"/StructTreeRoot",
            "image_format:jpeg": b"/Filter /DCTDecode",
            "vector_feature:shared-form-xobject": b"/Subtype /Form",
        }.items():
            if feature not in coverage or marker not in payload:
                raise MachineProfileError(
                    f"production PDF lacks required {feature!r} marker"
                )
        return
    for feature, marker in ADVERTISED_PDF_FEATURE_MARKERS.items():
        if feature in coverage and marker not in payload:
            raise MachineProfileError(
                f"advertised PDF feature {feature!r} is absent from {pdf}"
            )


def _verify_fixture(
    repository: Path,
    expected_path: Path,
    executable: Path,
    scratch: Path,
    *,
    runs: int,
    environment: Mapping[str, str],
    validators: Mapping[str, Draft202012Validator],
    expected_capabilities: bytes,
    mutool: str,
    pdftotext: str,
    pdfinfo: str,
) -> FixtureResult:
    expected = _load_json(expected_path)
    if (
        not isinstance(expected, dict)
        or expected.get("fixture_class") != "positive"
        or not isinstance(expected.get("expected"), dict)
        or expected["expected"].get("exit_code") != 0
    ):
        raise MachineProfileError(f"{expected_path}: release verification requires a positive fixture")
    _assert_fixture_resource_hashes(expected_path, expected)
    _assert_production_handoff(repository, expected, expected_capabilities)
    run_directories: list[Path] = []
    run_artifacts: list[dict[str, bytes]] = []
    for ordinal in range(runs):
        output = scratch / f"run-{ordinal + 1}"
        output.mkdir(parents=True)
        arguments = reproducibility._machine_fixture_arguments(expected, output)
        _run_checked([executable, *arguments], cwd=expected_path.parent, environment=environment)
        capabilities = _capabilities(
            executable, scratch / f"capabilities-hostile-{ordinal + 1}", environment
        )
        if capabilities != expected_capabilities:
            raise MachineProfileError("public capabilities bytes differ from the canonical fixture")
        (output / "capabilities.json").write_bytes(capabilities)
        artifacts: dict[str, bytes] = {}
        for kind, filename in ARTIFACT_FILES.items():
            path = output / filename
            if not path.is_file():
                raise MachineProfileError(f"build-package did not publish {kind}: {path}")
            artifacts[kind] = path.read_bytes()
        for kind, schema_name in ARTIFACT_SCHEMAS.items():
            _validate_instance(
                validators,
                schema_name,
                _load_json(output / ARTIFACT_FILES[kind]),
                f"runtime {kind}",
            )
        manifest = _load_json(output / "manifest.json")
        trace = _load_json(output / "trace.json")
        manifest_output = manifest.get("output") if isinstance(manifest, dict) else None
        if (
            not isinstance(manifest_output, dict)
            or manifest_output.get("bytes") != len(artifacts["pdf"])
            or manifest_output.get("sha256") != _sha256(artifacts["pdf"])
        ):
            raise MachineProfileError("manifest output facts do not bind the generated PDF")
        if not isinstance(manifest, dict) or not isinstance(trace, dict):
            raise MachineProfileError("machine trace or manifest root is not an object")
        _assert_profile_receipt_closure(expected, manifest, trace)
        _assert_production_vector_closure(expected, manifest, trace)
        _assert_table_layout_facts(expected, manifest)
        _assert_footnote_layout_facts(expected, manifest, trace)
        run_directories.append(output)
        run_artifacts.append(artifacts)

    baseline = run_artifacts[0]
    for ordinal, artifacts in enumerate(run_artifacts[1:], 2):
        for kind in ARTIFACT_FILES:
            if artifacts[kind] != baseline[kind]:
                raise MachineProfileError(
                    f"{kind} bytes differ between run 1 and run {ordinal}: "
                    f"{_sha256(baseline[kind])} != {_sha256(artifacts[kind])}"
                )

    check_diagnostics = scratch / "check-diagnostics.json"
    _run_checked(
        [executable, *_check_arguments(expected, check_diagnostics)],
        cwd=expected_path.parent,
        environment=environment,
    )
    _validate_instance(
        validators,
        "diagnostics.schema.json",
        _load_json(check_diagnostics),
        "check-package diagnostics",
    )
    if _load_json(check_diagnostics).get("diagnostics") != []:
        raise MachineProfileError("positive check-package emitted diagnostics")

    expected_pages = expected["expected"].get("page_count")
    expected_text = expected["expected"].get("normalized_extracted_text")
    if not isinstance(expected_pages, int) or expected_pages <= 0 or not isinstance(expected_text, str):
        raise MachineProfileError("positive fixture lacks external PDF expectations")
    try:
        differential = pdf_differential.verify_pdf_differential(
            [directory / "output.pdf" for directory in run_directories],
            expected_text=expected_text,
            expected_pages=expected_pages,
            mutool=mutool,
            pdftotext=pdftotext,
            pdfinfo=pdfinfo,
        )
    except (OSError, pdf_differential.PdfDifferentialError) as error:
        raise MachineProfileError(f"external PDF differential failed: {error}") from error
    structure_observations: list[dict[str, Any]] = []
    production_package: dict[str, Any] | None = None
    production_ledger: dict[str, Any] | None = None
    if expected.get("profile") == "typaxis.machine-pdf/production-book-1":
        production_package = _load_json(expected_path.parent / expected["package"])
        production_ledger = _load_json(expected_path.parent / "ledger.json")
    for directory in run_directories:
        _assert_advertised_pdf_features(expected, directory / "output.pdf")
        if production_package is not None and production_ledger is not None:
            try:
                structure_observations.append(
                    pdf_structure.verify_production_pdf_structure(
                        (directory / "output.pdf").read_bytes(),
                        production_package,
                        production_ledger,
                        expected_page_count=expected_pages,
                    )
                )
            except (OSError, pdf_structure.PdfValidationError) as error:
                raise MachineProfileError(
                    f"independent production PDF structure validation failed: {error}"
                ) from error
        _assert_table_zero_decoration(
            expected,
            directory / "output.pdf",
            repository=repository,
            environment=environment,
            mutool=mutool,
        )
        _assert_footnote_separator_paint(
            expected,
            _load_json(directory / "manifest.json"),
            directory / "output.pdf",
            repository=repository,
            environment=environment,
            mutool=mutool,
        )
    return FixtureResult(
        expected_path=expected_path,
        expected=expected,
        run_directories=tuple(run_directories),
        artifacts=baseline,
        differential=differential,
        structure=structure_observations[0] if structure_observations else None,
    )


def _resolve_tool(name: str, override: str | None = None) -> Path:
    candidate = override
    if candidate is not None and os.sep not in candidate and (
        os.altsep is None or os.altsep not in candidate
    ):
        candidate = shutil.which(candidate)
    if candidate is None:
        candidate = shutil.which(name)
    if candidate is None:
        raise MachineProfileError(f"required tool is unavailable: {name}")
    path = Path(candidate)
    if not path.is_absolute():
        path = Path.cwd() / path
    if not path.is_file():
        raise MachineProfileError(f"required tool is not a file: {path}")
    return path


def _tool_version(path: Path, arguments: Sequence[str]) -> str:
    try:
        completed = subprocess.run(
            [path, *arguments],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise MachineProfileError(f"cannot execute {path}: {error}") from error
    if completed.returncode != 0:
        raise MachineProfileError(f"cannot query {path.name} version")
    combined = completed.stdout + completed.stderr
    version = combined.decode("utf-8", "replace").strip().splitlines()
    if not version:
        raise MachineProfileError(f"{path.name} returned an empty version")
    return version[0]


def _tool_records(
    cargo: str,
    mutool: str | None,
    pdftotext: str | None,
    pdfinfo: str | None,
) -> tuple[list[dict[str, str]], dict[str, Path]]:
    paths = {
        "cargo": _resolve_tool("cargo", cargo),
        "mutool": _resolve_tool("mutool", mutool),
        "pdfinfo": _resolve_tool("pdfinfo", pdfinfo),
        "pdftotext": _resolve_tool("pdftotext", pdftotext),
        "python": Path(sys.executable).resolve(strict=True),
        "rustc": _resolve_tool("rustc"),
    }
    version_arguments = {
        "cargo": ["--version"],
        "mutool": ["-v"],
        "pdfinfo": ["-v"],
        "pdftotext": ["-v"],
        "python": ["--version"],
        "rustc": ["--version"],
    }
    records = [
        {
            "name": name,
            "sha256": _sha256(path.read_bytes()),
            "version": _tool_version(path, version_arguments[name]),
        }
        for name, path in sorted(paths.items())
    ]
    return records, paths


def _host_record(rustc: Path, repository: Path, environment: Mapping[str, str]) -> dict[str, str]:
    system = platform.system()
    host_os = {"Darwin": "macos", "Linux": "linux"}.get(system)
    if host_os is None:
        raise MachineProfileError(f"machine profile host evidence is unavailable on {system}")
    verbose = _run_capture([rustc, "-vV"], cwd=repository, environment=environment)
    host_lines = [
        line.split(":", 1)[1].strip()
        for line in verbose.decode("utf-8", "replace").splitlines()
        if line.startswith("host:")
    ]
    if len(host_lines) != 1:
        raise MachineProfileError("rustc -vV did not report exactly one host triple")
    return {
        "arch": platform.machine(),
        "os": host_os,
        "target_triple": host_lines[0],
    }


def _atomic_write(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            prefix=f".{path.name}.", suffix=".tmp", dir=path.parent, delete=False
        ) as output:
            temporary = Path(output.name)
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
        temporary = None
        if os.name != "nt":
            descriptor = os.open(path.parent, os.O_RDONLY)
            try:
                os.fsync(descriptor)
            finally:
                os.close(descriptor)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def verify_machine_profile(
    repository: Path,
    fixture: Path,
    *,
    runs: int = 2,
    cargo: str = "cargo",
    mutool: str | None = None,
    pdftotext: str | None = None,
    pdfinfo: str | None = None,
    evidence_directory: Path | None = None,
) -> Path:
    if runs < 2:
        raise MachineProfileError("--runs must be at least 2")
    root = release.repository_root(repository)
    environment = reproducibility.filtered_environment()
    validators = _schema_validators(root)
    expected_paths = _fixture_paths(root, fixture)
    tool_records, tool_paths = _tool_records(cargo, mutool, pdftotext, pdfinfo)
    host = _host_record(tool_paths["rustc"], root, environment)
    revision = reproducibility._resolve_commit(root, "HEAD", environment)
    source_files = reproducibility._listed_worktree_files(root, environment)
    source_snapshot = reproducibility._source_snapshot_sha256(root, source_files)
    cargo_lock = (root / "workspace/Cargo.lock").read_bytes()
    expected_capabilities = (root / "samples/machine-package/capabilities.json").read_bytes()

    with tempfile.TemporaryDirectory(
        prefix=reproducibility.MACHINE_TEMPORARY_PREFIX
    ) as raw_temporary:
        temporary = Path(raw_temporary)
        target = temporary / reproducibility.MACHINE_TARGET_DIRECTORY_NAMES[0]
        if target.exists():
            raise MachineProfileError("clean Cargo target unexpectedly exists")
        try:
            executable = reproducibility._build_machine_binary(
                root,
                target,
                cargo=os.fspath(tool_paths["cargo"]),
                environment=environment,
            )
        except reproducibility.ReproducibilityError as error:
            raise MachineProfileError(str(error)) from error
        version = _run_capture([executable, "--version"], cwd=root, environment=environment)
        binary_version = version.decode("utf-8", "strict").strip()
        if not binary_version.startswith("typaxis "):
            raise MachineProfileError(f"invalid Typaxis version output: {binary_version!r}")
        binary_sha256 = _sha256(executable.read_bytes())

        results = [
            _verify_fixture(
                root,
                expected_path,
                executable,
                temporary / f"fixture-{index + 1}",
                runs=runs,
                environment=environment,
                validators=validators,
                expected_capabilities=expected_capabilities,
                mutool=os.fspath(tool_paths["mutool"]),
                pdftotext=os.fspath(tool_paths["pdftotext"]),
                pdfinfo=os.fspath(tool_paths["pdfinfo"]),
            )
            for index, expected_path in enumerate(expected_paths)
        ]
        primary = next(
            (
                result
                for result in results
                if result.expected.get("fixture_id", "").endswith(".combined")
            ),
            results[0],
        )
        capabilities_instance = json.loads(primary.artifacts["capabilities"])
        requested_profiles = {
            result.expected.get("profile")
            for result in results
            if isinstance(result.expected.get("profile"), str)
        }
        _assert_profile_closure(capabilities_instance, requested_profiles)

        try:
            machine_reproducibility = reproducibility.verify_machine_reproducibility(
                root,
                primary.expected_path,
                revision="HEAD",
                cargo=os.fspath(tool_paths["cargo"]),
            )
        except (OSError, UnicodeError, reproducibility.ReproducibilityError) as error:
            raise MachineProfileError(f"machine reproducibility failed: {error}") from error
        if (
            machine_reproducibility.revision != revision
            or machine_reproducibility.source_snapshot_sha256 != source_snapshot
            or machine_reproducibility.binary_version != binary_version
            or machine_reproducibility.binary_sha256 != binary_sha256
            or machine_reproducibility.artifact_sha256
            != {kind: _sha256(payload) for kind, payload in primary.artifacts.items()}
        ):
            raise MachineProfileError(
                "machine reproducibility result does not bind the primary public run"
            )

        artifacts = [
            {"bytes": len(payload), "kind": kind, "sha256": _sha256(payload)}
            for kind, payload in sorted(primary.artifacts.items())
        ]
        checks = [
            {"name": "artifact_byte_identity", "result": "passed"},
            {"name": "build_package_run_1", "result": "passed"},
            {"name": "build_package_run_2", "result": "passed"},
            {"name": "capabilities_profile_closure", "result": "passed"},
            {"name": "capabilities_schema", "result": "passed"},
            {"name": "check_package", "result": "passed"},
            {"name": "clean_binary_build", "result": "passed"},
            {"name": "diagnostics_schema", "result": "passed"},
            {
                "detail": str(primary.differential.page_count),
                "name": "external_page_count",
                "result": "passed",
            },
            {
                "detail": primary.differential.extracted_text_sha256,
                "name": "external_poppler_text",
                "result": "passed",
            },
            {
                "detail": primary.differential.render_sha256,
                "name": "external_mupdf_raster",
                "result": "passed",
            },
            {"name": "manifest_schema", "result": "passed"},
            {"name": "machine_reproducibility", "result": "passed"},
            {"name": "profile_receipt_closure", "result": "passed"},
            {"name": "trace_schema", "result": "passed"},
        ]
        required_checks = set(REQUIRED_CHECKS)
        if primary.structure is not None:
            checks.append(
                {
                    "detail": _sha256(canonical_json_bytes(primary.structure)),
                    "name": PRODUCTION_REQUIRED_CHECK,
                    "result": "passed",
                }
            )
            required_checks.add(PRODUCTION_REQUIRED_CHECK)
        if {check["name"] for check in checks} != required_checks:
            raise MachineProfileError("internal evidence check set is incomplete")
        evidence = {
            "artifacts": artifacts,
            "binary": {"sha256": binary_sha256, "version": binary_version},
            "checks": sorted(checks, key=lambda check: check["name"]),
            "contract": EVIDENCE_CONTRACT,
            "fixture": {
                "expected_sha256": _sha256(primary.expected_path.read_bytes()),
                "fixture_id": primary.expected["fixture_id"],
                "resources": primary.expected["resource_hashes"],
            },
            "host": host,
            "reproducibility": machine_reproducibility.as_json(),
            "result": "passed",
            "source": {
                "cargo_lock_sha256": _sha256(cargo_lock),
                "revision": revision,
                "snapshot_sha256": source_snapshot,
            },
            "tools": tool_records,
        }
        _validate_instance(
            validators,
            "machine-profile-evidence.schema.json",
            evidence,
            "host evidence",
        )
        encoded = canonical_json_bytes(evidence)

    evidence_root = evidence_directory
    if evidence_root is None:
        evidence_root = root / "target/machine-e2e/host-evidence"
    elif not evidence_root.is_absolute():
        evidence_root = root / evidence_root
    evidence_path = evidence_root / f"{host['target_triple']}.json"
    _atomic_write(evidence_path, encoded)
    return evidence_path


def require_host_evidence(
    repository: Path,
    evidence_directory: Path,
    required_hosts: Sequence[str],
) -> dict[str, Path]:
    if not required_hosts:
        raise MachineProfileError("at least one --required-host is required")
    if len(set(required_hosts)) != len(required_hosts) or any(
        host not in {"linux", "macos"} for host in required_hosts
    ):
        raise MachineProfileError("required hosts must be unique linux/macos values")
    root = release.repository_root(repository)
    directory = evidence_directory
    if not directory.is_absolute():
        directory = root / directory
    if not directory.is_dir():
        raise MachineProfileError(f"host evidence directory does not exist: {directory}")
    validators = _schema_validators(root)
    environment = reproducibility.filtered_environment()
    revision = reproducibility._resolve_commit(root, "HEAD", environment)
    observed: dict[str, Path] = {}
    common_snapshot: str | None = None
    common_fixture: str | None = None
    common_artifacts: dict[str, str] | None = None
    for path in sorted(directory.glob("**/*.json")):
        evidence = _load_json(path)
        _validate_instance(
            validators,
            "machine-profile-evidence.schema.json",
            evidence,
            os.fspath(path),
        )
        if path.read_bytes() != canonical_json_bytes(evidence):
            raise MachineProfileError(f"host evidence is not canonical JCS: {path}")
        host = evidence["host"]["os"]
        triple = evidence["host"]["target_triple"]
        if path.name != f"{triple}.json":
            raise MachineProfileError(f"host evidence filename does not match target triple: {path}")
        if host in observed:
            raise MachineProfileError(f"duplicate host evidence for {host}")
        if evidence["result"] != "passed" or any(
            check["result"] != "passed" for check in evidence["checks"]
        ):
            raise MachineProfileError(f"host evidence reports a failed gate: {path}")
        required_checks = set(REQUIRED_CHECKS)
        if evidence["fixture"]["fixture_id"].startswith("production-book-1."):
            required_checks.add(PRODUCTION_REQUIRED_CHECK)
        if {check["name"] for check in evidence["checks"]} != required_checks:
            raise MachineProfileError(f"host evidence has an incomplete check set: {path}")
        if {tool["name"] for tool in evidence["tools"]} != REQUIRED_TOOLS:
            raise MachineProfileError(f"host evidence has an incomplete tool set: {path}")
        if {artifact["kind"] for artifact in evidence["artifacts"]} != set(ARTIFACT_FILES):
            raise MachineProfileError(f"host evidence has an incomplete artifact set: {path}")
        if (
            evidence["source"]["revision"] != revision
            or evidence["reproducibility"]["revision"] != revision
        ):
            raise MachineProfileError(f"host evidence is stale for revision {revision}: {path}")
        snapshot = evidence["source"]["snapshot_sha256"]
        fixture_hash = evidence["fixture"]["expected_sha256"]
        artifact_hashes = {
            artifact["kind"]: artifact["sha256"] for artifact in evidence["artifacts"]
        }
        if common_snapshot is None:
            common_snapshot = snapshot
            common_fixture = fixture_hash
            common_artifacts = artifact_hashes
        elif (
            snapshot != common_snapshot
            or fixture_hash != common_fixture
            or artifact_hashes != common_artifacts
        ):
            raise MachineProfileError("host evidence does not describe one identical source/fixture/artifact set")
        observed[host] = path
    missing = sorted(set(required_hosts) - set(observed))
    if missing:
        raise MachineProfileError("missing required host evidence: " + ", ".join(missing))
    return {host: observed[host] for host in required_hosts}


def _parse_arguments(arguments: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    fixture_group = parser.add_mutually_exclusive_group()
    fixture_group.add_argument("--fixture", type=Path)
    fixture_group.add_argument("--matrix", type=Path)
    parser.add_argument("--runs", type=int, default=2)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--mutool")
    parser.add_argument("--pdftotext")
    parser.add_argument("--pdfinfo")
    parser.add_argument("--evidence-directory", type=Path)
    parser.add_argument(
        "--require-external-tools",
        action="store_true",
        help="fail if MuPDF or Poppler tools are unavailable (host evidence always does)",
    )
    parser.add_argument("--require-host-evidence", type=Path)
    parser.add_argument("--required-host", action="append", default=[])
    return parser.parse_args(arguments)


def main(arguments: Sequence[str] | None = None) -> int:
    options = _parse_arguments(sys.argv[1:] if arguments is None else arguments)
    try:
        if options.require_host_evidence is not None:
            observed = require_host_evidence(
                options.repository,
                options.require_host_evidence,
                options.required_host,
            )
            for host, path in observed.items():
                print(f"{host} {path}")
            return 0
        selected_fixture = options.matrix or options.fixture
        if selected_fixture is None:
            raise MachineProfileError("--fixture or --matrix is required outside aggregation mode")
        # Host evidence is defined to include the external gate. The explicit
        # flag documents release intent; absence never turns a missing tool
        # into a successful skip.
        evidence = verify_machine_profile(
            options.repository,
            selected_fixture,
            runs=options.runs,
            cargo=options.cargo,
            mutool=options.mutool,
            pdftotext=options.pdftotext,
            pdfinfo=options.pdfinfo,
            evidence_directory=options.evidence_directory,
        )
    except (
        MachineProfileError,
        OSError,
        UnicodeError,
        release.ReleaseError,
    ) as error:
        print(f"machine profile verification error: {error}", file=sys.stderr)
        return 1
    print(f"host evidence {evidence}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
