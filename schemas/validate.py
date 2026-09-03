#!/usr/bin/env python3
"""Offline contract validation for the bundled Typaxis schemas and fixtures."""

from __future__ import annotations

import copy
import base64
import datetime
import hashlib
import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urldefrag, urljoin

try:
    from jsonschema import Draft202012Validator
    from referencing import Registry, Resource
except ImportError as error:  # pragma: no cover - dependency guidance
    raise SystemExit(
        "schemas/validate.py requires jsonschema>=4.18 and referencing"
    ) from error


SCHEMA_DIR = Path(__file__).resolve().parent
FROZEN_SCHEMA_DIR = SCHEMA_DIR / "1.0"
PREVIOUS_SCHEMA_DIR = SCHEMA_DIR / "1.1"
FROZEN_1_2_SCHEMA_DIR = SCHEMA_DIR / "1.2"
VERSIONED_CURRENT_SCHEMA_DIR = SCHEMA_DIR / "1.3"
PRIVATE_M4_SCHEMA_DIR = SCHEMA_DIR / "1.4"
REPOSITORY_ROOT = SCHEMA_DIR.parent
MINIMAL_DIR = REPOSITORY_ROOT / "samples" / "minimal"
CONFORMANCE_DIR = REPOSITORY_ROOT / "samples" / "conformance"
COMPATIBILITY_DIR = REPOSITORY_ROOT / "samples" / "compatibility"
INVALID_DIR = REPOSITORY_ROOT / "samples" / "invalid"
MACHINE_FIXTURE_DIR = REPOSITORY_ROOT / "samples" / "machine-package"
STAGING_STYLE_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "basic-document-1"
    / "machine-block-styles"
)
STAGING_LIST_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "basic-document-1"
    / "machine-list"
)
STAGING_PAGE_BREAK_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "basic-document-1"
    / "machine-page-break"
)
STAGING_FIGURE_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "basic-document-1"
    / "machine-figure"
)
STAGING_LINK_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "basic-document-1"
    / "machine-link"
)
STAGING_HEADER_FOOTER_FIXTURE_ROOT = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "header-footer-1"
)
STAGING_COLUMNS_FIXTURE_ROOT = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "columns-1"
)
STAGING_FLOAT_FIXTURE_ROOT = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "float-1"
)
STAGING_SEMANTIC_CONTAINER_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "production-book-1"
    / "semantic-container"
)
STAGING_SAFE_VECTOR_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "production-book-1"
    / "vector-media"
)
STAGING_MATH_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "production-book-1"
    / "math"
)
STAGING_PRECOMPOSED_VECTOR_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "production-book-1"
    / "precomposed-vector"
)
STAGING_BOOK_NAVIGATION_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "production-book-1"
    / "book-navigation"
)
STAGING_ACCESSIBILITY_FIXTURE_DIR = (
    MACHINE_FIXTURE_DIR
    / "staging"
    / "production-book-1"
    / "accessibility"
)
JSON_SAFE_INTEGER_MAX = 9_007_199_254_740_991
MAX_AST_NESTING_DEPTH = 64
MAX_FONT_SUBSET_TAGS = 26**6
MAX_DOCUMENT_PACKAGE_BYTES = JSON_SAFE_INTEGER_MAX
MAX_JSON_NESTING_DEPTH = 256
JCS_GOLDEN_PATH = MINIMAL_DIR / "jcs-golden.json"
FROZEN_SCHEMA_SHA256 = {
    "build-manifest.schema.json": "138c72e08e47957f76bb530cd0956097a8ba354818aa27fbc7070012e984ca5e",
    "common.schema.json": "94a2631d90c028f977241fef91b51a3b361fd6d023d3ea02556a5f4dd2dbd695",
    "diagnostics.schema.json": "2730062972c35194acd51bb057d16bb1cb8962682f670b9e7c0732b11fc55aa9",
    "display-list.schema.json": "8125198467006ec79b484d21b6a945478946986b01d1b4bfeac28f46c58798bc",
    "document-package.schema.json": "2976bba8247b5cc2db5220356c942d9b14b36a5b4a201d60296b1c46c9dc17d4",
    "layout-trace.schema.json": "00ad4bb4e9fc427db7219d4fef1492e432b735dad909764125a952ddd195b1b2",
    "package-config.schema.json": "8b9450a52c050e893b76e548fb219f95f6492939c50856d8536a298ded6fc145",
}
PREVIOUS_SCHEMA_SHA256 = {
    "build-manifest.schema.json": "4ebcc6cb0f25e7cd82a7905cdca37990cd60e013009da7fbcbd999201c0f60b3",
    "common.schema.json": "223d524c21f6aafa444944e4445816dba6edcb51099ecad6953176970c387f7a",
    "diagnostics.schema.json": "c53510cbe8474328344268e4e838043e8e62258a88a29cd4c4c81b654eb525c1",
    "display-list.schema.json": "90e14174ce464bb5a3e63f6d34aa96a4f3ee6c659bbb0ba8d587d99eac0e7d8b",
    "document-package.schema.json": "e573dff00f252bb723ea29e2faadf9a7a47d2248cbcc5756f439ed7935ccaa6c",
    "layout-trace.schema.json": "f70448b5fda23d3b743079e1a4477f79c6dbdb98d33021eee3e7c75aa49eb8a9",
    "machine-capabilities.schema.json": "7ce8f98e2193f1d81f256b44458737ecdec3aaccdc92c2df62b77e204bb14dea",
    "machine-fixture-expectation.schema.json": "f0de1ea48b4de1110ff483c40593a6c132ade4af183c81150401519cf610f7c8",
    "machine-fixture-matrix.schema.json": "1e306b97e3f8dd506633973787cbeb76f424893f86702d8d8c3b42960e7e6cff",
    "machine-profile-evidence.schema.json": "eb3609a6b197c3d9b0cc4550d245085e5a630f8043f111de6609405467462c83",
    "package-config.schema.json": "f5c5c85a7e50f01d316a5bf4b298680f75ba57bbb92c0ec059479ec618475e16",
}
FROZEN_1_2_SCHEMA_SHA256 = {
    "build-manifest.schema.json": "1e7119f9e49c600e0236b171a3cc992d41b2a7ab323a49936969f03eb48a8612",
    "common.schema.json": "b328201dee245f9a094e63639c5af9359ec874119a3906f4b8df6185946f86a1",
    "diagnostics.schema.json": "fb8cf302ae581a7e361f1d00776a3002be92bb6eeffaa73eeb127dceef1c78cb",
    "display-list.schema.json": "c1c7527e6e5de6437109e0461092d5ddca5888e24667c227ecf1027ad2ea97b0",
    "document-package.schema.json": "de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a",
    "layout-trace.schema.json": "2952df047bb58c921ba3e577977968f5a3f029387ef87f50f40d469e9f91a874",
    "machine-block-style-manifest.schema.json": "83c08c829b943a3f8d3890a8317d9259789851f68ed592732558ec6ca9118b71",
    "machine-capabilities.schema.json": "c3dbaaf9bf3a9940ce4957288e1d67047b8742a8b58581e8a9fba654291263c6",
    "machine-figure-manifest.schema.json": "67cb5f330dcd30319c6f8aa95c61c632e063b755337b71a019a758b90c7ec16c",
    "machine-fixture-expectation.schema.json": "924ffa9e45a64b20032ef94c0118067ab067ffd3d59b06c2c8f87b8840c452ff",
    "machine-fixture-matrix.schema.json": "747d12b0cfa527ba89c873ad76d9f905cb887685c971af0e5781d4dd5321f88e",
    "machine-footnote-manifest.schema.json": "1dc01f7f8163e7c6506230059f0a142f84685f257fe38c6cd6a95c4d58e8f3fd",
    "machine-forced-page-break-manifest.schema.json": "4f55ac91f2a32803ae3555c41cc8a61b1b97407f5519bd094b177ba4b00f8a25",
    "machine-forced-page-break-trace.schema.json": "a1bf9f8bda0b8e7bbdd1360a4b56d72f99ea6b003372fb5dd5631e2ef8ccdf54",
    "machine-link-manifest.schema.json": "0489fdd2dd903234a8408197f35b410d18ba861582a6f761643851052c365dca",
    "machine-list-manifest.schema.json": "0fc53d7e703b75be348e18043d7f978e05fbd3fa586eb6618edf2ad25c2fef4e",
    "machine-profile-evidence.schema.json": "36937f83bdac31de3e66604f50e1f1eca3abe39e7d8f8c7ca3121bcb40f5b829",
    "machine-table-manifest.schema.json": "0b4ca658ae7cd0d5c044ff13d0fe0e6b5c2f70089353b5b385754204efb8ba0e",
    "package-config.schema.json": "7ea50014c09cfdd73873089514d59990497ba106aab969b00781c790bbcc1f9c",
}

POSITIVE_FIXTURES = {
    MINIMAL_DIR / "typaxis.toml": "package-config.schema.json",
    MINIMAL_DIR / "document-package.json": "document-package.schema.json",
    MINIMAL_DIR / "display-list.json": "display-list.schema.json",
    MINIMAL_DIR / "layout-trace.json": "layout-trace.schema.json",
    MINIMAL_DIR / "build-manifest.json": "build-manifest.schema.json",
    MINIMAL_DIR / "diagnostics.json": "diagnostics.schema.json",
    CONFORMANCE_DIR / "document-rich.json": "document-package.schema.json",
    CONFORMANCE_DIR / "display-text.json": "display-list.schema.json",
    CONFORMANCE_DIR / "manifest-numeric-boundaries.json": "build-manifest.schema.json",
    CONFORMANCE_DIR / "config-font-count-boundary.json": "package-config.schema.json",
    CONFORMANCE_DIR / "display-rtl.json": "display-list.schema.json",
    CONFORMANCE_DIR / "document-style-fallback.json": "document-package.schema.json",
    CONFORMANCE_DIR / "machine-capabilities.json": "machine-capabilities.schema.json",
}
POSITIVE_CROSS_FIXTURES = (CONFORMANCE_DIR / "cross-generated-sites.json",)

INVALID_SCHEMA_BY_PREFIX = {
    "capabilities-": "machine-capabilities.schema.json",
    "config-": "package-config.schema.json",
    "diagnostics-": "diagnostics.schema.json",
    "display-": "display-list.schema.json",
    "document-": "document-package.schema.json",
    "manifest-": "build-manifest.schema.json",
    "trace-": "layout-trace.schema.json",
}

RULE_ID = re.compile(r"^[A-Z][A-Z0-9_]*$")
DIAGNOSTIC_CODE = re.compile(r"^(?:P1|T2|S3|F4|L5|G6|R7|D8|I9)[0-9]{3}$")
STYLE_SELECTOR = re.compile(
    r"^(?:paragraph|heading|list|table|figure|page_break)"
    r"(?:\.[A-Za-z_][A-Za-z0-9_-]*)*$"
)
CLASS_TOKEN = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")
DECLARATION_NAME = re.compile(r"^[a-z][a-z0-9_-]*$")
STYLE_PROPERTY_NAMES = {"font_family", "font_size", "line_height", "page"}
DISPLAY_OPS = {
    "save",
    "restore",
    "concat_transform",
    "clip_path",
    "fill_path",
    "stroke_path",
    "draw_glyph_run",
    "draw_image",
}
PATH_KEYS = {
    "move_to": {"verb", "x", "y"},
    "line_to": {"verb", "x", "y"},
    "curve_to": {"verb", "x1", "y1", "x2", "y2", "x3", "y3"},
    "close": {"verb"},
}

PATCH_FIXTURE_BASES = {
    "machine_capabilities": (
        CONFORMANCE_DIR / "machine-capabilities.json",
        "machine-capabilities.schema.json",
    ),
    "package_config": (MINIMAL_DIR / "typaxis.toml", "package-config.schema.json"),
    "document_package": (
        CONFORMANCE_DIR / "document-rich.json",
        "document-package.schema.json",
    ),
    "display_list": (CONFORMANCE_DIR / "display-text.json", "display-list.schema.json"),
    "layout_trace": (MINIMAL_DIR / "layout-trace.json", "layout-trace.schema.json"),
    "build_manifest": (
        MINIMAL_DIR / "build-manifest.json",
        "build-manifest.schema.json",
    ),
}
CROSS_ARTIFACT_NAMES = {"config", "document", "display", "trace", "manifest"}
FRAME_KIND_ORDER = {"body": 0, "header": 1, "footer": 2, "footnote": 3}
GENERATION_KIND_ORDER = {
    "page_reference": 0,
    "counter": 1,
    "list_marker": 2,
    "footnote_marker": 3,
    "discretionary": 4,
}


class ValidationFailure(Exception):
    """Raised for a contract-suite consistency failure."""


def reject_duplicate_members(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ValidationFailure(f"duplicate JSON member: {key!r}")
        value[key] = item
    return value


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicate_members)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        raise ValidationFailure(f"{path}: invalid UTF-8 JSON: {error}") from error


def load_instance(path: Path) -> Any:
    if path.suffix == ".json":
        return load_json(path)
    if path.suffix == ".toml":
        try:
            with path.open("rb") as source:
                return tomllib.load(source)
        except tomllib.TOMLDecodeError as error:
            raise ValidationFailure(f"{path}: invalid TOML: {error}") from error
    raise ValidationFailure(f"{path}: unsupported fixture extension")


def utf8_sort_key(value: str) -> bytes:
    try:
        return value.encode("utf-8")
    except UnicodeEncodeError as error:
        raise ValidationFailure("canonical string contains an unpaired surrogate") from error


def contains_non_scalar_string(value: Any) -> bool:
    stack = [value]
    while stack:
        current = stack.pop()
        if isinstance(current, str):
            if any(0xD800 <= ord(character) <= 0xDFFF for character in current):
                return True
        elif isinstance(current, dict):
            for key, child in current.items():
                stack.extend((key, child))
        elif isinstance(current, list):
            stack.extend(current)
    return False


def is_canonical_string_list(value: Any) -> bool:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        return False
    return len(value) == len(set(value)) and value == sorted(value, key=utf8_sort_key)


def utf8_boundary_set(value: str) -> tuple[int, set[int]] | None:
    try:
        encoded = value.encode("utf-8")
    except UnicodeEncodeError:
        return None
    boundaries = {0}
    byte_offset = 0
    for character in value:
        byte_offset += len(character.encode("utf-8"))
        boundaries.add(byte_offset)
    return len(encoded), boundaries


def materialize_fixture_value(value: Any, label: str, source: Any = None) -> Any:
    if isinstance(value, dict) and set(value) == {"$copy"}:
        path = value["$copy"]
        if not isinstance(path, list) or any(type(token) not in {str, int} for token in path):
            raise ValidationFailure(f"{label}: $copy must be a string/integer path")
        current = source
        try:
            for token in path:
                current = current[token]
        except (KeyError, IndexError, TypeError) as error:
            raise ValidationFailure(f"{label}: $copy path does not exist") from error
        return copy.deepcopy(current)
    if isinstance(value, dict) and "$repeat" in value:
        if set(value) != {"$repeat", "count", "prefix"}:
            raise ValidationFailure(
                f"{label}: generated string must contain $repeat, count, and prefix"
            )
        repeated = value["$repeat"]
        count = value["count"]
        prefix = value["prefix"]
        if (
            not isinstance(repeated, str)
            or type(count) is not int
            or count < 0
            or not isinstance(prefix, str)
        ):
            raise ValidationFailure(f"{label}: invalid generated string operands")
        return prefix + repeated * count
    if isinstance(value, dict):
        return {
            key: materialize_fixture_value(child, f"{label}.{key}", source)
            for key, child in value.items()
        }
    if isinstance(value, list):
        return [
            materialize_fixture_value(child, f"{label}[{index}]", source)
            for index, child in enumerate(value)
        ]
    return copy.deepcopy(value)


def apply_fixture_mutations(document: Any, mutations: Any, label: str) -> Any:
    if not isinstance(mutations, list) or not mutations:
        raise ValidationFailure(f"{label}: mutations must be a non-empty array")
    result = copy.deepcopy(document)
    for mutation_index, mutation in enumerate(mutations):
        mutation_label = f"{label}: mutation {mutation_index}"
        if not isinstance(mutation, dict) or set(mutation) != {"path", "value"}:
            raise ValidationFailure(
                f"{mutation_label}: mutation must contain exactly path and value"
            )
        path = mutation["path"]
        if (
            not isinstance(path, list)
            or not path
            or any(type(token) not in {str, int} for token in path)
        ):
            raise ValidationFailure(f"{mutation_label}: path must contain string/integer tokens")
        current = result
        try:
            for token in path[:-1]:
                current = current[token]
            final_token = path[-1]
            if (
                isinstance(current, list)
                and type(final_token) is int
                and final_token == len(current)
            ):
                current.append(materialize_fixture_value(mutation["value"], mutation_label, result))
            else:
                current[final_token] = materialize_fixture_value(
                    mutation["value"], mutation_label, result
                )
        except (KeyError, IndexError, TypeError) as error:
            raise ValidationFailure(f"{mutation_label}: path does not exist") from error
    return result


def materialize_patch_fixture(
    raw_fixture: Any, expected_schema: str, label: str
) -> Any:
    if not isinstance(raw_fixture, dict) or "$fixture" not in raw_fixture:
        return raw_fixture
    if set(raw_fixture) != {"$fixture", "mutations"}:
        raise ValidationFailure(
            f"{label}: patch fixture must contain exactly $fixture and mutations"
        )
    fixture_name = raw_fixture["$fixture"]
    fixture_base = PATCH_FIXTURE_BASES.get(fixture_name)
    if fixture_base is None:
        raise ValidationFailure(f"{label}: unknown patch fixture base {fixture_name!r}")
    base_path, base_schema = fixture_base
    if base_schema != expected_schema:
        raise ValidationFailure(
            f"{label}: patch base {fixture_name!r} does not match {expected_schema}"
        )
    return apply_fixture_mutations(
        load_instance(base_path), raw_fixture["mutations"], label
    )


def walk_references(value: Any, path: str = ""):
    if isinstance(value, dict):
        if "$ref" in value:
            yield path or "/", value["$ref"]
        for key, child in value.items():
            yield from walk_references(child, f"{path}/{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from walk_references(child, f"{path}/{index}")


def resolve_json_pointer(document: Any, fragment: str) -> None:
    if not fragment:
        return
    if not fragment.startswith("/"):
        raise ValidationFailure(f"unsupported non-pointer schema fragment: #{fragment}")
    current = document
    for encoded_token in fragment[1:].split("/"):
        token = unquote(encoded_token).replace("~1", "/").replace("~0", "~")
        try:
            current = current[int(token)] if isinstance(current, list) else current[token]
        except (KeyError, IndexError, ValueError, TypeError) as error:
            raise ValidationFailure(f"missing JSON Pointer target: #{fragment}") from error


def validate_references(schemas: dict[str, Any]) -> int:
    by_id = {schema["$id"]: schema for schema in schemas.values()}
    if len(by_id) != len(schemas):
        raise ValidationFailure("schema $id values must be unique")

    reference_count = 0
    for filename, schema in schemas.items():
        schema_id = schema["$id"]
        for pointer, reference in walk_references(schema):
            reference_count += 1
            absolute = urljoin(schema_id, reference)
            resource_id, fragment = urldefrag(absolute)
            target = by_id.get(resource_id)
            if target is None:
                raise ValidationFailure(
                    f"{filename}{pointer}: unregistered $ref resource {resource_id!r}"
                )
            try:
                resolve_json_pointer(target, fragment)
            except ValidationFailure as error:
                raise ValidationFailure(f"{filename}{pointer}: {error}") from error
    return reference_count


def schema_errors(validator: Draft202012Validator, instance: Any) -> list[str]:
    schema_id = validator.schema.get("$id")
    if (
        isinstance(schema_id, str)
        and schema_id.endswith("/document-package.schema.json")
        and canonical_ast_nesting_depth(instance) > MAX_AST_NESTING_DEPTH
    ):
        # The semantic depth rule is authoritative. Do not enter jsonschema's
        # recursive `$ref` evaluator with a document the profile must reject.
        return []
    errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.absolute_path))
    return [f"{error.json_path}: {error.message}" for error in errors]


def check_safe_integers(value: Any, path: str = "$") -> None:
    stack = [(value, path)]
    while stack:
        current, current_path = stack.pop()
        if isinstance(current, bool):
            continue
        if isinstance(current, int):
            if not -JSON_SAFE_INTEGER_MAX <= current <= JSON_SAFE_INTEGER_MAX:
                raise ValidationFailure(
                    f"{current_path}: integer is outside the JCS exact range"
                )
            continue
        if isinstance(current, dict):
            for key, child in reversed(tuple(current.items())):
                stack.append((child, f"{current_path}.{key}"))
        elif isinstance(current, list):
            for index in range(len(current) - 1, -1, -1):
                stack.append((current[index], f"{current_path}[{index}]"))


def jcs_bytes(value: Any) -> bytes:
    """Serialize the effective config's JSON data model using its JCS subset.

    Configuration member names are ASCII and values contain only strings,
    booleans, arrays, objects, and exact-range integers, so Python's compact,
    sorted JSON encoding is byte-identical to RFC 8785 for this data model.
    """

    if contains_non_scalar_string(value):
        raise ValidationFailure("JCS input contains an unpaired Unicode surrogate")
    check_safe_integers(value)
    return json.dumps(
        value,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def verify_file_record(base: Path, record: dict[str, Any], label: str) -> None:
    path = base / record["uri"]
    if not path.is_file():
        raise ValidationFailure(f"{label}: recorded file does not exist: {path}")
    contents = path.read_bytes()
    if len(contents) != record["bytes"]:
        raise ValidationFailure(f"{label}: byte count does not match {path}")
    digest = hashlib.sha256(contents).hexdigest()
    if digest != record["sha256"]:
        raise ValidationFailure(f"{label}: SHA-256 does not match {path}")


def validate_jcs_golden(effective_config: dict[str, Any]) -> int:
    golden = load_json(JCS_GOLDEN_PATH)
    if (
        not isinstance(golden, dict)
        or golden.get("algorithm") != "rfc8785-jcs/base64/1"
        or set(golden) != {"algorithm", "entries"}
        or not isinstance(golden.get("entries"), list)
    ):
        raise ValidationFailure(f"{JCS_GOLDEN_PATH}: malformed JCS golden root")
    expected_values: dict[str, Any] = {"effective-config": effective_config}
    for path in sorted(MINIMAL_DIR.glob("*.json")):
        if path != JCS_GOLDEN_PATH:
            expected_values[path.name] = load_json(path)
    entries = golden["entries"]
    names = [entry.get("artifact") for entry in entries if isinstance(entry, dict)]
    if (
        len(names) != len(entries)
        or len(names) != len(set(names))
        or names != sorted(names, key=utf8_sort_key)
        or set(names) != set(expected_values)
    ):
        raise ValidationFailure(f"{JCS_GOLDEN_PATH}: noncanonical or incomplete entry set")
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {
            "artifact", "canonical_jcs_base64", "sha256"
        }:
            raise ValidationFailure(f"{JCS_GOLDEN_PATH}: malformed JCS golden entry")
        expected = jcs_bytes(expected_values[entry["artifact"]])
        encoded = entry["canonical_jcs_base64"]
        try:
            observed = base64.b64decode(encoded, validate=True)
        except (ValueError, TypeError) as error:
            raise ValidationFailure(
                f"{JCS_GOLDEN_PATH}: invalid canonical_jcs_base64"
            ) from error
        if observed != expected:
            raise ValidationFailure(
                f"{JCS_GOLDEN_PATH}: JCS bytes differ for {entry['artifact']}"
            )
        if entry["sha256"] != hashlib.sha256(expected).hexdigest():
            raise ValidationFailure(
                f"{JCS_GOLDEN_PATH}: JCS SHA-256 differs for {entry['artifact']}"
            )
    return len(entries)


def pdf_dictionary_tokens(body: bytes) -> bytes:
    """Remove comments/strings and stop before a top-level stream payload."""

    output = bytearray()
    index = 0
    literal_depth = 0
    while index < len(body):
        byte = body[index]
        if literal_depth:
            if byte == 0x5C:
                index += 2
                continue
            if byte == 0x28:
                literal_depth += 1
            elif byte == 0x29:
                literal_depth -= 1
            index += 1
            continue
        if byte == 0x25:
            newline = body.find(b"\n", index)
            index = len(body) if newline < 0 else newline + 1
            output.append(0x20)
            continue
        if byte == 0x28:
            literal_depth = 1
            output.append(0x20)
            index += 1
            continue
        if body.startswith(b"<<", index) or body.startswith(b">>", index):
            output.extend(body[index:index + 2])
            index += 2
            continue
        if byte == 0x3C:
            close = body.find(b">", index + 1)
            index = len(body) if close < 0 else close + 1
            output.append(0x20)
            continue
        if body.startswith(b"stream", index) and (
            index == 0 or body[index - 1] in b"\x00\t\n\x0c\r "
        ):
            break
        output.append(byte)
        index += 1
    return bytes(output)


def parse_classic_pdf_facts(contents: bytes) -> tuple[int, int]:
    start_match = re.search(rb"startxref\s+(\d+)\s+%%EOF\s*$", contents)
    if start_match is None:
        raise ValidationFailure("PDF lacks a terminal classic startxref")
    xref_offset = int(start_match.group(1))
    if not contents.startswith(b"xref", xref_offset):
        raise ValidationFailure("PDF startxref does not point to a classic xref table")
    cursor = xref_offset + 4

    def read_line() -> bytes:
        nonlocal cursor
        while cursor < len(contents) and contents[cursor] in b"\r\n":
            cursor += 1
        end = contents.find(b"\n", cursor)
        if end < 0:
            raise ValidationFailure("truncated PDF xref table")
        line = contents[cursor:end].rstrip(b"\r")
        cursor = end + 1
        return line

    in_use: dict[int, tuple[int, int]] = {}
    while True:
        while cursor < len(contents) and contents[cursor] in b"\r\n":
            cursor += 1
        if contents.startswith(b"trailer", cursor):
            cursor += len(b"trailer")
            break
        subsection = read_line()
        match = re.fullmatch(rb"(\d+)\s+(\d+)", subsection)
        if match is None:
            raise ValidationFailure("malformed PDF xref subsection")
        first, count = map(int, match.groups())
        for object_id in range(first, first + count):
            entry = read_line()
            entry_match = re.fullmatch(rb"(\d{10})\s(\d{5})\s([nf])\s?", entry)
            if entry_match is None:
                raise ValidationFailure("malformed PDF xref entry")
            offset, generation, state = entry_match.groups()
            if state == b"n":
                if object_id in in_use:
                    raise ValidationFailure("duplicate in-use PDF xref object")
                in_use[object_id] = (int(offset), int(generation))

    trailer_end = contents.find(b"startxref", cursor)
    if trailer_end < 0:
        raise ValidationFailure("PDF trailer is not terminated")
    trailer = pdf_dictionary_tokens(contents[cursor:trailer_end])
    root_match = re.search(rb"/Root\s+(\d+)\s+(\d+)\s+R\b", trailer)
    size_match = re.search(rb"/Size\s+(\d+)\b", trailer)
    if root_match is None or size_match is None:
        raise ValidationFailure("PDF trailer lacks Root or Size")
    root_ref = (int(root_match.group(1)), int(root_match.group(2)))
    if int(size_match.group(1)) <= max(in_use, default=0):
        raise ValidationFailure("PDF trailer Size does not cover xref objects")

    object_bodies: dict[int, bytes] = {}
    offsets = sorted((offset, object_id, generation) for object_id, (offset, generation) in in_use.items())
    for offset_index, (offset, object_id, generation) in enumerate(offsets):
        header = f"{object_id} {generation} obj".encode()
        if not contents.startswith(header, offset):
            raise ValidationFailure("PDF xref offset does not match its object header")
        next_offset = offsets[offset_index + 1][0] if offset_index + 1 < len(offsets) else xref_offset
        object_slice = contents[offset + len(header):next_offset]
        endobj = object_slice.rfind(b"endobj")
        if endobj < 0:
            raise ValidationFailure("PDF indirect object lacks endobj before the next xref offset")
        object_bodies[object_id] = pdf_dictionary_tokens(object_slice[:endobj])

    root_id, root_generation = root_ref
    if in_use.get(root_id, (None, None))[1] != root_generation:
        raise ValidationFailure("PDF trailer Root is not an in-use object")
    root_body = object_bodies[root_id]
    if re.search(rb"/Type\s*/Catalog\b", root_body) is None:
        raise ValidationFailure("PDF Root is not a Catalog")
    pages_match = re.search(rb"/Pages\s+(\d+)\s+(\d+)\s+R\b", root_body)
    if pages_match is None:
        raise ValidationFailure("PDF Catalog lacks a Pages reference")

    visiting: set[int] = set()

    def count_page_leaves(object_id: int, generation: int) -> int:
        if in_use.get(object_id, (None, None))[1] != generation or object_id not in object_bodies:
            raise ValidationFailure("PDF page tree references a missing object")
        if object_id in visiting:
            raise ValidationFailure("PDF page tree contains a cycle")
        visiting.add(object_id)
        body = object_bodies[object_id]
        if re.search(rb"/Type\s*/Page\b", body):
            visiting.remove(object_id)
            return 1
        if re.search(rb"/Type\s*/Pages\b", body) is None:
            raise ValidationFailure("PDF page tree child is neither Page nor Pages")
        kids_match = re.search(rb"/Kids\s*\[(.*?)\]", body, re.DOTALL)
        count_match = re.search(rb"/Count\s+(\d+)\b", body)
        if kids_match is None or count_match is None:
            raise ValidationFailure("PDF Pages node lacks Kids or Count")
        kid_refs = [
            (int(match.group(1)), int(match.group(2)))
            for match in re.finditer(rb"(\d+)\s+(\d+)\s+R\b", kids_match.group(1))
        ]
        leaves = sum(count_page_leaves(kid_id, kid_generation) for kid_id, kid_generation in kid_refs)
        if leaves != int(count_match.group(1)):
            raise ValidationFailure("PDF Pages Count does not match reachable leaf pages")
        visiting.remove(object_id)
        return leaves

    page_count = count_page_leaves(int(pages_match.group(1)), int(pages_match.group(2)))
    return page_count, len(in_use)


def walk_objects(value: Any):
    stack = [value]
    while stack:
        current = stack.pop()
        if isinstance(current, dict):
            yield current
            stack.extend(reversed(tuple(current.values())))
        elif isinstance(current, list):
            stack.extend(reversed(current))


def is_portable_path(value: Any) -> bool:
    if not isinstance(value, str) or not value or value.startswith("/"):
        return False
    if "\\" in value or ":" in value:
        return False
    if any(ord(character) < 0x20 or ord(character) == 0x7F for character in value):
        return False
    return all(component not in {"", ".", ".."} for component in value.split("/"))


def is_nonempty_scalar_name(value: Any) -> bool:
    return (
        isinstance(value, str)
        and bool(value)
        and not value.isspace()
        and not contains_non_scalar_string(value)
        and not any(ord(character) < 0x20 or ord(character) == 0x7F for character in value)
    )


def rect_contains(parent: Any, child: Any) -> bool:
    if not isinstance(child, dict) or not isinstance(parent, dict):
        return False
    if not all(
        type(rect.get(field)) is int
        for rect in (child, parent)
        for field in ("x", "y", "width", "height")
    ):
        return False
    return (
        child["x"] >= parent["x"]
        and child["y"] >= parent["y"]
        and child["x"] + child["width"] <= parent["x"] + parent["width"]
        and child["y"] + child["height"] <= parent["y"] + parent["height"]
    )


def typed_document_nodes_with_depth(document: Any):
    """Yield typed semantic nodes iteratively in canonical preorder."""

    stack = [("document", document, 1)]
    while stack:
        node_type, node, depth = stack.pop()
        if not isinstance(node, dict):
            continue
        yield node, depth
        child_depth = depth + 1
        if node_type == "document":
            footnotes = node.get("footnotes", [])
            if isinstance(footnotes, list):
                stack.extend(
                    ("footnote", footnote, child_depth)
                    for footnote in reversed(footnotes)
                )
            blocks = node.get("blocks", [])
            if isinstance(blocks, list):
                stack.extend(
                    ("block", block, child_depth) for block in reversed(blocks)
                )
        elif node_type == "block":
            kind = node.get("kind")
            if kind in {"paragraph", "heading"}:
                children = node.get("children", [])
                if isinstance(children, list):
                    stack.extend(
                        ("inline", child, child_depth)
                        for child in reversed(children)
                    )
            elif kind == "list":
                items = node.get("items", [])
                if isinstance(items, list):
                    stack.extend(
                        ("list_item", item, child_depth)
                        for item in reversed(items)
                    )
            elif kind == "table":
                body = node.get("body", [])
                if isinstance(body, list):
                    stack.extend(
                        ("table_row", row, child_depth) for row in reversed(body)
                    )
                head = node.get("head", [])
                if isinstance(head, list):
                    stack.extend(
                        ("table_row", row, child_depth) for row in reversed(head)
                    )
            elif kind == "figure":
                caption = node.get("caption", [])
                if isinstance(caption, list):
                    stack.extend(
                        ("block", block, child_depth) for block in reversed(caption)
                    )
        elif node_type == "inline":
            if node.get("kind") in {"emphasis", "strong", "link"}:
                children = node.get("children", [])
                if isinstance(children, list):
                    stack.extend(
                        ("inline", child, child_depth)
                        for child in reversed(children)
                    )
        elif node_type in {"footnote", "list_item", "table_cell"}:
            blocks = node.get("blocks", [])
            if isinstance(blocks, list):
                stack.extend(
                    ("block", block, child_depth) for block in reversed(blocks)
                )
        elif node_type == "table_row":
            cells = node.get("cells", [])
            if isinstance(cells, list):
                stack.extend(
                    ("table_cell", cell, child_depth) for cell in reversed(cells)
                )


def typed_document_preorder(document: Any) -> list[dict[str, Any]]:
    """Return the Profile 1.0 typed preorder, independent of JSON member order."""

    return [node for node, _depth in typed_document_nodes_with_depth(document)]


def canonical_style_inheritance_depth(package: Any) -> tuple[int, bool]:
    """Return the deepest valid style chain and whether any chain is cyclic."""

    if not isinstance(package, dict):
        return 0, False
    rules = package.get("style_sheet", {}).get("rules", [])
    if not isinstance(rules, list):
        return 0, False
    parents: dict[str, str | None] = {}
    for rule in rules:
        if not isinstance(rule, dict) or not isinstance(rule.get("style_id"), str):
            continue
        parent = rule.get("extends")
        parents[rule["style_id"]] = parent if isinstance(parent, str) else None

    depths: dict[str, int] = {}
    max_depth = 0
    for start in parents:
        if start in depths:
            max_depth = max(max_depth, depths[start])
            continue
        path: list[str] = []
        positions: dict[str, int] = {}
        current: str | None = start
        while current is not None and current in parents and current not in depths:
            if current in positions:
                return max_depth, True
            positions[current] = len(path)
            path.append(current)
            current = parents[current]
        depth = depths.get(current, 0) if current is not None else 0
        for style_id in reversed(path):
            depth += 1
            depths[style_id] = depth
            max_depth = max(max_depth, depth)
    return max_depth, False


def canonical_ast_nesting_depth(package: Any) -> int:
    """Return the max typed Document or style-inheritance depth (roots are 1)."""

    if not isinstance(package, dict):
        return 0
    document_depth = max(
        (depth for _node, depth in typed_document_nodes_with_depth(package.get("document"))),
        default=0,
    )
    style_depth, style_cycle = canonical_style_inheritance_depth(package)
    return document_depth if style_cycle else max(document_depth, style_depth)


def typed_document_paths(document: Any) -> dict[int, tuple[int, ...]]:
    """Map NodeId to its typed child path without using JSON member order."""

    paths: dict[int, tuple[int, ...]] = {}
    if not isinstance(document, dict):
        return paths

    stack = [("document", document, ())]
    while stack:
        node_type, node, path = stack.pop()
        if not isinstance(node, dict):
            continue
        if type(node.get("node_id")) is int:
            paths[node["node_id"]] = path
        if node_type == "document":
            blocks = node.get("blocks", [])
            block_count = len(blocks) if isinstance(blocks, list) else 0
            footnotes = node.get("footnotes", [])
            if isinstance(footnotes, list):
                for index in range(len(footnotes) - 1, -1, -1):
                    stack.append(
                        ("footnote", footnotes[index], (block_count + index,))
                    )
            if isinstance(blocks, list):
                for index in range(len(blocks) - 1, -1, -1):
                    stack.append(("block", blocks[index], (index,)))
        elif node_type == "block":
            kind = node.get("kind")
            if kind in {"paragraph", "heading"}:
                children = node.get("children", [])
                if isinstance(children, list):
                    for index in range(len(children) - 1, -1, -1):
                        stack.append(("inline", children[index], (*path, index)))
            elif kind == "list":
                items = node.get("items", [])
                if isinstance(items, list):
                    for index in range(len(items) - 1, -1, -1):
                        stack.append(("list_item", items[index], (*path, index)))
            elif kind == "table":
                head = node.get("head", [])
                head_count = len(head) if isinstance(head, list) else 0
                body = node.get("body", [])
                if isinstance(body, list):
                    for index in range(len(body) - 1, -1, -1):
                        stack.append(
                            ("table_row", body[index], (*path, head_count + index))
                        )
                if isinstance(head, list):
                    for index in range(len(head) - 1, -1, -1):
                        stack.append(("table_row", head[index], (*path, index)))
            elif kind == "figure":
                caption = node.get("caption", [])
                if isinstance(caption, list):
                    for index in range(len(caption) - 1, -1, -1):
                        stack.append(("block", caption[index], (*path, index)))
        elif node_type == "inline":
            if node.get("kind") in {"emphasis", "strong", "link"}:
                children = node.get("children", [])
                if isinstance(children, list):
                    for index in range(len(children) - 1, -1, -1):
                        stack.append(("inline", children[index], (*path, index)))
        elif node_type in {"footnote", "list_item", "table_cell"}:
            blocks = node.get("blocks", [])
            if isinstance(blocks, list):
                for index in range(len(blocks) - 1, -1, -1):
                    stack.append(("block", blocks[index], (*path, index)))
        elif node_type == "table_row":
            cells = node.get("cells", [])
            if isinstance(cells, list):
                for index in range(len(cells) - 1, -1, -1):
                    stack.append(("table_cell", cells[index], (*path, index)))
    return paths


def typed_document_node_kinds(document: Any) -> dict[int, str]:
    """Map NodeId to its typed semantic kind without JSON-member-order inference."""

    kinds: dict[int, str] = {}
    stack = [("document", document)]
    while stack:
        node_type, node = stack.pop()
        if not isinstance(node, dict):
            continue
        node_id = node.get("node_id")
        if type(node_id) is int:
            kinds[node_id] = node.get("kind") if node_type in {"block", "inline"} else node_type
        if node_type == "document":
            footnotes = node.get("footnotes", [])
            blocks = node.get("blocks", [])
            if isinstance(footnotes, list):
                stack.extend(("footnote", item) for item in reversed(footnotes))
            if isinstance(blocks, list):
                stack.extend(("block", item) for item in reversed(blocks))
        elif node_type == "block":
            kind = node.get("kind")
            if kind in {"paragraph", "heading"}:
                children = node.get("children", [])
                if isinstance(children, list):
                    stack.extend(("inline", item) for item in reversed(children))
            elif kind == "list":
                items = node.get("items", [])
                if isinstance(items, list):
                    stack.extend(("list_item", item) for item in reversed(items))
            elif kind == "table":
                body = node.get("body", [])
                head = node.get("head", [])
                if isinstance(body, list):
                    stack.extend(("table_row", item) for item in reversed(body))
                if isinstance(head, list):
                    stack.extend(("table_row", item) for item in reversed(head))
            elif kind == "figure":
                caption = node.get("caption", [])
                if isinstance(caption, list):
                    stack.extend(("block", item) for item in reversed(caption))
        elif node_type == "inline":
            if node.get("kind") in {"emphasis", "strong", "link"}:
                children = node.get("children", [])
                if isinstance(children, list):
                    stack.extend(("inline", item) for item in reversed(children))
        elif node_type in {"footnote", "list_item", "table_cell"}:
            blocks = node.get("blocks", [])
            if isinstance(blocks, list):
                stack.extend(("block", item) for item in reversed(blocks))
        elif node_type == "table_row":
            cells = node.get("cells", [])
            if isinstance(cells, list):
                stack.extend(("table_cell", item) for item in reversed(cells))
    return kinds


def canonical_ast_node_count(package: dict[str, Any]) -> int:
    """Count Profile 1.0 syntax AST nodes represented in the package."""

    count = len(typed_document_preorder(package.get("document", {})))
    rules = package.get("style_sheet", {}).get("rules", [])
    if not isinstance(rules, list):
        return count
    for rule in rules:
        if not isinstance(rule, dict):
            continue
        declarations = rule.get("declarations", [])
        if not isinstance(declarations, list):
            continue
        for declaration in declarations:
            if isinstance(declaration, dict):
                count += 1
                if isinstance(declaration.get("value"), dict):
                    count += 1
    return count


def expected_generated_sites(package: dict[str, Any]) -> dict[tuple[Any, ...], dict[str, Any]]:
    """Derive Profile 1.0 generated-text sites from the typed Document preorder."""

    sites: dict[tuple[Any, ...], dict[str, Any]] = {}
    root = package.get("document", {})
    for node in typed_document_preorder(root):
        owner = node.get("node_id")
        kind = node.get("kind")
        if type(owner) is not int:
            continue
        generation_kind = None
        target = None
        if kind == "reference":
            generation_kind = "page_reference" if node.get("format") == "page" else "counter"
            target = node.get("target")
        elif kind == "footnote_reference":
            generation_kind = "footnote_marker"
            target = node.get("footnote_id")
        elif kind == "soft_break":
            generation_kind = "discretionary"
        if generation_kind is not None:
            key = generated_key_tuple({
                "owner": owner,
                "generation_kind": generation_kind,
                "owner_local_ordinal": 0,
            })
            if key is not None:
                sites[key] = {"generation_kind": generation_kind, "target": target}
        if kind == "list":
            for item in node.get("items", []):
                item_owner = item.get("node_id") if isinstance(item, dict) else None
                key = generated_key_tuple({
                    "owner": item_owner,
                    "generation_kind": "list_marker",
                    "owner_local_ordinal": 0,
                })
                if key is not None:
                    sites[key] = {"generation_kind": "list_marker", "target": None}
    for footnote in root.get("footnotes", []) if isinstance(root, dict) else []:
        owner = footnote.get("node_id") if isinstance(footnote, dict) else None
        key = generated_key_tuple({
            "owner": owner,
            "generation_kind": "footnote_marker",
            "owner_local_ordinal": 0,
        })
        if key is not None:
            sites[key] = {
                "generation_kind": "footnote_marker",
                "target": footnote.get("footnote_id"),
            }
    return sites


def display_rule_ids(
    instance: dict[str, Any],
    allowed_uri_schemes: set[str] | None = None,
    max_uri_bytes: int | None = None,
) -> set[str]:
    rules: set[str] = set()
    pages = instance.get("pages", [])
    if isinstance(pages, list):
        if not pages:
            rules.add("DISPLAY_EMPTY_PAGES")
        if any(
            not isinstance(page, dict) or page.get("page_index") != page_index
            for page_index, page in enumerate(pages)
        ):
            rules.add("DISPLAY_PAGE_INDEX")

    text_buffers = instance.get("text_buffers", [])
    display_text_info: dict[int, tuple[int, set[int]]] = {}
    referenced_display_text_ids: set[int] = set()
    duplicate_display_text_ids: set[Any] = set()
    if isinstance(text_buffers, list):
        text_ids = [
            text_buffer.get("text_id")
            for text_buffer in text_buffers
            if isinstance(text_buffer, dict)
        ]
        duplicate_display_text_ids = {
            text_id for text_id in text_ids if text_ids.count(text_id) > 1
        }
        if text_ids and (
            any(type(text_id) is not int for text_id in text_ids)
            or text_ids != list(range(len(text_ids)))
        ):
            rules.add("DISPLAY_TEXT_BUFFER_INDEX")
        origin_keys: list[tuple[Any, ...]] = []
        for text_buffer in text_buffers:
            if not isinstance(text_buffer, dict):
                continue
            origin = text_buffer.get("origin")
            if not isinstance(origin, dict):
                continue
            if origin.get("kind") == "parsed" and type(origin.get("text_buffer_id")) is int:
                origin_keys.append((0, origin["text_buffer_id"]))
            elif origin.get("kind") == "generated":
                key = generated_key_tuple(origin.get("key"))
                if key is not None:
                    origin_keys.append((1, *key))
        if len(origin_keys) != len(set(origin_keys)) or origin_keys != sorted(origin_keys):
            rules.add("DISPLAY_TEXT_ORIGIN_ORDER")
        for text_buffer in text_buffers:
            if not isinstance(text_buffer, dict):
                continue
            text_id = text_buffer.get("text_id")
            utf8 = text_buffer.get("utf8")
            if (
                type(text_id) is int
                and text_id not in duplicate_display_text_ids
                and isinstance(utf8, str)
            ):
                boundary_info = utf8_boundary_set(utf8)
                if boundary_info is None:
                    rules.add("DISPLAY_UTF8_BOUNDARY")
                else:
                    display_text_info[text_id] = boundary_info

    font_instances = instance.get("font_instances", [])
    known_font_instance_ids: set[int] = set()
    referenced_font_instance_ids: set[int] = set()
    if isinstance(font_instances, list):
        instance_ids = [
            font_instance.get("font_instance_id")
            for font_instance in font_instances
            if isinstance(font_instance, dict)
        ]
        valid_instance_ids = all(type(instance_id) is int for instance_id in instance_ids)
        if valid_instance_ids and len(instance_ids) == len(set(instance_ids)):
            if instance_ids != list(range(len(instance_ids))):
                rules.add("DISPLAY_FONT_INSTANCE_INDEX")
        elif instance_ids:
            rules.add("DISPLAY_FONT_INSTANCE_INDEX")
        face_ids = [
            font_instance.get("font_face_id")
            for font_instance in font_instances
            if isinstance(font_instance, dict)
        ]
        if all(type(face_id) is int for face_id in face_ids):
            if len(face_ids) != len(set(face_ids)):
                rules.add("DISPLAY_DUPLICATE_FONT_FACE")
            elif face_ids != sorted(face_ids):
                rules.add("DISPLAY_FONT_INSTANCE_ORDER")
        known_font_instance_ids = {
            instance_id for instance_id in instance_ids if type(instance_id) is int
        }

    destinations_in_order = instance.get("destinations", [])
    if isinstance(destinations_in_order, list):
        destination_anchors = [
            destination.get("anchor_id")
            for destination in destinations_in_order
            if isinstance(destination, dict)
        ]
        if len(destination_anchors) != len(set(destination_anchors)):
            rules.add("DISPLAY_DESTINATION_ANCHOR")
        elif all(isinstance(anchor, str) for anchor in destination_anchors) and (
            destination_anchors != sorted(destination_anchors, key=utf8_sort_key)
        ):
            rules.add("DISPLAY_DESTINATION_ORDER")
        known_pages = {
            page.get("page_index") for page in pages if isinstance(page, dict)
        }
        if any(
            isinstance(destination, dict)
            and destination.get("page_index") not in known_pages
            for destination in destinations_in_order
        ):
            rules.add("DISPLAY_DESTINATION_PAGE")
        pages_by_id = {
            page.get("page_index"): page for page in pages if isinstance(page, dict)
        }
        for destination in destinations_in_order:
            if not isinstance(destination, dict):
                continue
            page = pages_by_id.get(destination.get("page_index"))
            view = destination.get("view", {})
            point = view.get("point", {}) if isinstance(view, dict) else {}
            if (
                isinstance(page, dict)
                and isinstance(view, dict)
                and view.get("kind") == "xyz"
                and isinstance(point, dict)
                and all(type(point.get(axis)) is int for axis in ("x", "y"))
                and type(page.get("width")) is int
                and type(page.get("height")) is int
                and (
                    point["x"] < 0
                    or point["y"] < 0
                    or point["x"] > page["width"]
                    or point["y"] > page["height"]
                )
            ):
                rules.add("DISPLAY_DESTINATION_BOUNDS")
            if (
                isinstance(page, dict)
                and isinstance(view, dict)
                and view.get("kind") == "fit_width"
                and view.get("top") is not None
                and type(view.get("top")) is int
                and type(page.get("height")) is int
                and not 0 <= view["top"] <= page["height"]
            ):
                rules.add("DISPLAY_DESTINATION_BOUNDS")

    destinations = {
        destination.get("anchor_id")
        for destination in destinations_in_order
        if isinstance(destination, dict)
    }

    glyph_run_ids: list[Any] = []
    for page in instance.get("pages", []):
        if not isinstance(page, dict):
            continue
        page_width = page.get("width")
        page_height = page.get("height")
        graphics_depth = 0
        for command in page.get("commands", []):
            if not isinstance(command, dict):
                continue
            if command.get("op") == "save":
                graphics_depth += 1
            elif command.get("op") == "restore":
                if graphics_depth == 0:
                    rules.add("DISPLAY_GRAPHICS_UNDERFLOW")
                else:
                    graphics_depth -= 1
        if graphics_depth != 0:
            rules.add("DISPLAY_GRAPHICS_UNBALANCED")

        for annotation in page.get("annotations", []):
            if not isinstance(annotation, dict):
                continue
            target = annotation.get("target", {})
            if isinstance(target, dict):
                if target.get("kind") == "internal" and target.get("anchor_id") not in destinations:
                    rules.add("DISPLAY_UNRESOLVED_ANCHOR")
                if target.get("kind") == "uri":
                    uri = target.get("uri")
                    scheme = uri.split(":", 1)[0] if isinstance(uri, str) and ":" in uri else None
                    if (
                        scheme not in {"http", "https", "mailto", "tel"}
                        or (
                            allowed_uri_schemes is not None
                            and scheme not in allowed_uri_schemes
                        )
                    ):
                        rules.add("DISPLAY_URI_SCHEME")
                    if (
                        isinstance(uri, str)
                        and max_uri_bytes is not None
                        and len(uri.encode("utf-8")) > max_uri_bytes
                    ):
                        rules.add("DISPLAY_URI_LIMIT")
            rect = annotation.get("rect", {})
            if (
                isinstance(rect, dict)
                and all(isinstance(rect.get(key), int) for key in ("x", "y", "width", "height"))
                and isinstance(page_width, int)
                and isinstance(page_height, int)
                and (
                    rect["x"] < 0
                    or rect["y"] < 0
                    or rect["x"] + rect["width"] > page_width
                    or rect["y"] + rect["height"] > page_height
                )
            ):
                rules.add("DISPLAY_ANNOTATION_BOUNDS")

        for command in page.get("commands", []):
            if not isinstance(command, dict):
                continue
            operation = command.get("op")
            if operation not in DISPLAY_OPS:
                rules.add("DISPLAY_UNKNOWN_OP")
                continue
            if operation == "concat_transform":
                matrix = command.get("matrix")
                if not isinstance(matrix, dict) or set(matrix) != {
                    "a_16_16",
                    "b_16_16",
                    "c_16_16",
                    "d_16_16",
                    "e",
                    "f",
                }:
                    rules.add("DISPLAY_TRANSFORM_SHAPE")
            if operation == "draw_glyph_run":
                glyph_run_ids.append(command.get("run_id"))
                resource_id = command.get("font_instance_id")
                if type(resource_id) is not int or not 0 <= resource_id <= 0xFFFF_FFFF:
                    rules.add("DISPLAY_RESOURCE_ID")
                elif resource_id not in known_font_instance_ids:
                    rules.add("DISPLAY_UNKNOWN_FONT_INSTANCE")
                else:
                    referenced_font_instance_ids.add(resource_id)
                if "fill" not in command:
                    rules.add("DISPLAY_TEXT_PAINT")
                run_span = command.get("text_span", {})

                def validate_display_span(span: Any) -> bool:
                    if not isinstance(span, dict):
                        return False
                    text_id = span.get("text_id")
                    start = span.get("start_byte")
                    end = span.get("end_byte")
                    if text_id in duplicate_display_text_ids:
                        return False
                    if type(text_id) is not int or text_id not in display_text_info:
                        rules.add("DISPLAY_UNKNOWN_TEXT")
                        return False
                    referenced_display_text_ids.add(text_id)
                    if type(start) is not int or type(end) is not int:
                        return False
                    text_length, boundaries = display_text_info[text_id]
                    if start > end or start < 0 or end > text_length:
                        rules.add("DISPLAY_TEXT_SPAN_BOUNDS")
                        return False
                    if start not in boundaries or end not in boundaries:
                        rules.add("DISPLAY_UTF8_BOUNDARY")
                        return False
                    return True

                run_span_valid = validate_display_span(run_span)
                unicode_spans: list[dict[str, Any]] = []
                cluster_spans_valid = True
                glyphs = command.get("glyphs", [])
                glyph_count = len(glyphs) if isinstance(glyphs, list) else 0
                clusters = command.get("clusters", [])
                covered_glyphs: list[int] = []
                for logical_ordinal, cluster in enumerate(clusters):
                    if not isinstance(cluster, dict):
                        continue
                    if cluster.get("logical_ordinal") != logical_ordinal:
                        rules.add("DISPLAY_CLUSTER_INDEX")
                    glyph_start = cluster.get("glyph_start")
                    glyph_end = cluster.get("glyph_end")
                    if (
                        type(glyph_start) is not int or type(glyph_end) is not int
                        or not 0 <= glyph_start < glyph_end <= glyph_count
                    ):
                        rules.add("DISPLAY_GLYPH_PARTITION")
                    else:
                        covered_glyphs.extend(range(glyph_start, glyph_end))
                    if cluster.get("extraction") != "unicode":
                        continue
                    span = cluster.get("text_span", {})
                    if not validate_display_span(span):
                        cluster_spans_valid = False
                    elif isinstance(span, dict):
                        if span.get("start_byte") == span.get("end_byte"):
                            rules.add("DISPLAY_EMPTY_UNICODE_CLUSTER")
                            cluster_spans_valid = False
                        unicode_spans.append(span)

                if sorted(covered_glyphs) != list(range(glyph_count)) or len(covered_glyphs) != len(set(covered_glyphs)):
                    rules.add("DISPLAY_GLYPH_PARTITION")

                if run_span_valid and cluster_spans_valid:
                    overlap = any(
                        current.get("text_id") == previous.get("text_id")
                        and current.get("start_byte") < previous.get("end_byte")
                        for previous, current in zip(unicode_spans, unicode_spans[1:])
                    )
                    if overlap:
                        rules.add("DISPLAY_CLUSTER_OVERLAP")
                    else:
                        expected_start = run_span.get("start_byte")
                        coverage_valid = True
                        for span in unicode_spans:
                            if (
                                span.get("text_id") != run_span.get("text_id")
                                or span.get("start_byte") != expected_start
                                or span.get("end_byte") > run_span.get("end_byte")
                            ):
                                coverage_valid = False
                                break
                            expected_start = span.get("end_byte")
                        if expected_start != run_span.get("end_byte"):
                            coverage_valid = False
                        if not coverage_valid:
                            rules.add("DISPLAY_CLUSTER_COVERAGE")
            if operation == "draw_image":
                resource_id = command.get("image_id")
                if type(resource_id) is not int or not 0 <= resource_id <= 0xFFFF_FFFF:
                    rules.add("DISPLAY_RESOURCE_ID")
            if operation == "stroke_path":
                dash = command.get("stroke", {}).get("dash", {})
                values = dash.get("array") if isinstance(dash, dict) else None
                if isinstance(values, list) and values and all(value == 0 for value in values):
                    rules.add("DISPLAY_DASH_ALL_ZERO")
            if operation in {"clip_path", "fill_path", "stroke_path"}:
                path = command.get("path")
                arity_valid = isinstance(path, list) and bool(path)
                if arity_valid:
                    for verb in path:
                        if (
                            not isinstance(verb, dict)
                            or verb.get("verb") not in PATH_KEYS
                            or set(verb) != PATH_KEYS[verb["verb"]]
                        ):
                            arity_valid = False
                            break
                if not arity_valid:
                    rules.add("DISPLAY_PATH_ARITY")
                    continue

                open_subpath = False
                drawable_in_subpath = False
                any_drawable = False
                valid_state = path[0]["verb"] == "move_to"
                for verb in path:
                    kind = verb["verb"]
                    if kind == "move_to":
                        open_subpath = True
                        drawable_in_subpath = False
                    elif kind in {"line_to", "curve_to"}:
                        if not open_subpath:
                            valid_state = False
                        drawable_in_subpath = True
                        any_drawable = True
                    elif kind == "close":
                        if not open_subpath or not drawable_in_subpath:
                            valid_state = False
                        open_subpath = False
                        drawable_in_subpath = False
                if not valid_state or not any_drawable:
                    rules.add("DISPLAY_PATH_STATE")
    if glyph_run_ids and (
        any(type(run_id) is not int for run_id in glyph_run_ids)
        or glyph_run_ids != list(range(len(glyph_run_ids)))
    ):
        rules.add("DISPLAY_GLYPH_RUN_INDEX")
    if (
        not duplicate_display_text_ids
        and not rules.intersection({"DISPLAY_EMPTY_PAGES", "DISPLAY_TEXT_BUFFER_INDEX", "DISPLAY_TEXT_ORIGIN_ORDER", "DISPLAY_UNKNOWN_TEXT", "DISPLAY_TEXT_SPAN_BOUNDS", "DISPLAY_UTF8_BOUNDARY"})
        and set(display_text_info) != referenced_display_text_ids
    ):
        rules.add("DISPLAY_UNUSED_TEXT_BUFFER")
    if not rules.intersection(
        {"DISPLAY_EMPTY_PAGES", "DISPLAY_RESOURCE_ID", "DISPLAY_UNKNOWN_FONT_INSTANCE", "DISPLAY_FONT_INSTANCE_INDEX", "DISPLAY_FONT_INSTANCE_ORDER", "DISPLAY_DUPLICATE_FONT_FACE"}
    ) and known_font_instance_ids != referenced_font_instance_ids:
        rules.add("DISPLAY_UNUSED_FONT_INSTANCE")
    return rules


def document_rule_ids(
    instance: dict[str, Any],
    allowed_uri_schemes: set[str] | None = None,
    max_uri_bytes: int | None = None,
    max_ast_nesting_depth: int | None = None,
) -> set[str]:
    rules: set[str] = set()
    document = instance.get("document", {})
    if (
        type(max_ast_nesting_depth) is int
        and canonical_ast_nesting_depth(instance) > max_ast_nesting_depth
    ):
        return {"CROSS_LIMIT_AST_NESTING_DEPTH"}
    document_objects = list(walk_objects(document))
    typed_nodes = typed_document_preorder(document)
    footnotes_for_order = (
        [footnote for footnote in document.get("footnotes", []) if isinstance(footnote, dict)]
        if isinstance(document, dict)
        else []
    )
    footnote_ids_for_order = [footnote.get("footnote_id") for footnote in footnotes_for_order]
    footnotes_canonical = (
        all(isinstance(footnote_id, str) for footnote_id in footnote_ids_for_order)
        and len(footnote_ids_for_order) == len(set(footnote_ids_for_order))
        and footnote_ids_for_order == sorted(footnote_ids_for_order, key=utf8_sort_key)
    )

    for value in document_objects:
        classes = value.get("classes")
        if isinstance(classes, list):
            valid_classes = all(
                isinstance(class_name, str)
                and CLASS_TOKEN.fullmatch(class_name) is not None
                for class_name in classes
            )
            if not valid_classes:
                rules.add("STYLE_CLASS_TOKEN")
            elif not is_canonical_string_list(classes):
                rules.add("STYLE_CLASS_ORDER")

        if value.get("kind") == "list" and isinstance(value.get("ordered"), bool):
            start = value.get("start")
            if value["ordered"]:
                if type(start) is not int or not 1 <= start <= 0xFFFF_FFFF:
                    rules.add("DOC_LIST_START")
                else:
                    items = value.get("items")
                    if (
                        isinstance(items, list)
                        and items
                        and start + len(items) - 1 > 0xFFFF_FFFF
                    ):
                        rules.add("DOC_LIST_MARKER_OVERFLOW")
            elif start is not None:
                rules.add("DOC_LIST_START")
            if value.get("items") == []:
                rules.add("DOC_EMPTY_LIST")

        if (
            value.get("kind") == "table"
            and value.get("head") == []
            and value.get("body") == []
        ):
            rules.add("DOC_EMPTY_TABLE")

    node_ids = [value.get("node_id") for value in typed_nodes]
    if all(type(node_id) is int for node_id in node_ids):
        if len(node_ids) != len(set(node_ids)):
            rules.add("DOC_DUPLICATE_NODE")
        elif footnotes_canonical and node_ids != list(range(len(node_ids))):
            rules.add("DOC_NODE_INDEX")

    source_records = [
        source for source in instance.get("sources", []) if isinstance(source, dict)
    ]
    source_ids = [source.get("source_id") for source in source_records]
    if all(type(source_id) is int for source_id in source_ids) and len(source_ids) != len(set(source_ids)):
        rules.add("DOC_DUPLICATE_SOURCE_ID")
    elif all(type(source_id) is int for source_id in source_ids) and source_ids != list(range(len(source_ids))):
        rules.add("DOC_SOURCE_INDEX")
    source_lengths = {
        source.get("source_id"): source.get("utf8_byte_length")
        for source in source_records
        if type(source.get("source_id")) is int
        and type(source.get("utf8_byte_length")) is int
    }
    for value in walk_objects(instance):
        if {"source_id", "start_byte", "end_byte"} <= set(value):
            source_id = value.get("source_id")
            start = value.get("start_byte")
            end = value.get("end_byte")
            if type(start) is not int or type(end) is not int:
                continue
            if start > end:
                rules.add("DOC_SPAN_REVERSED")
                continue
            if source_id not in source_lengths:
                rules.add("DOC_UNKNOWN_SOURCE")
            elif start < 0 or end > source_lengths[source_id]:
                rules.add("DOC_SOURCE_SPAN_BOUNDS")

    text_buffers = [
        buffer for buffer in instance.get("text_buffers", []) if isinstance(buffer, dict)
    ]
    text_ids_in_order = [buffer.get("text_id") for buffer in text_buffers]
    duplicate_text_ids = {
        text_id
        for text_id in text_ids_in_order
        if text_ids_in_order.count(text_id) > 1
    }
    if duplicate_text_ids:
        rules.add("DOC_DUPLICATE_TEXT_ID")
    elif all(type(text_id) is int for text_id in text_ids_in_order) and (
        text_ids_in_order != list(range(len(text_ids_in_order)))
    ):
        rules.add("DOC_TEXT_INDEX")
    text_info: dict[int, tuple[int, set[int]]] = {}
    for buffer in text_buffers:
        text_id = buffer.get("text_id")
        utf8 = buffer.get("utf8")
        if (
            type(text_id) is int
            and text_id not in duplicate_text_ids
            and isinstance(utf8, str)
        ):
            boundary_info = utf8_boundary_set(utf8)
            if boundary_info is None:
                rules.add("DOC_UTF8_BOUNDARY")
            else:
                text_info[text_id] = boundary_info

    for value in document_objects:
        text_span = value.get("text_span")
        if not isinstance(text_span, dict):
            continue
        text_id = text_span.get("text_id")
        start = text_span.get("start_byte")
        end = text_span.get("end_byte")
        if text_id in duplicate_text_ids:
            continue
        if text_id not in text_info:
            rules.add("DOC_UNKNOWN_TEXT")
            continue
        if type(start) is not int or type(end) is not int:
            continue
        text_length, boundaries = text_info[text_id]
        if start > end or start < 0 or end > text_length:
            rules.add("DOC_TEXT_SPAN_BOUNDS")
        elif start not in boundaries or end not in boundaries:
            rules.add("DOC_UTF8_BOUNDARY")

    for buffer in text_buffers:
        expected_start = 0
        coverage_valid = True
        ranges_in_bounds = True
        utf8 = buffer.get("utf8")
        boundary_info = utf8_boundary_set(utf8) if isinstance(utf8, str) else None
        for mapping in buffer.get("mappings", []):
            if not isinstance(mapping, dict):
                continue
            text_range = mapping.get("text_range", {})
            start = text_range.get("start_byte") if isinstance(text_range, dict) else None
            end = text_range.get("end_byte") if isinstance(text_range, dict) else None
            if type(start) is not int or type(end) is not int:
                continue
            if start == end:
                rules.add("TEXT_MAP_EMPTY_SEGMENT")
            if start != expected_start:
                coverage_valid = False
            expected_start = end
            if boundary_info is not None:
                text_length, boundaries = boundary_info
                if start > end or start < 0 or end > text_length:
                    rules.add("DOC_TEXT_SPAN_BOUNDS")
                    ranges_in_bounds = False
                elif start not in boundaries or end not in boundaries:
                    rules.add("DOC_UTF8_BOUNDARY")
            source_span = mapping.get("source_span")
            kind = mapping.get("kind")
            if kind == "inserted" and source_span is not None:
                rules.add("TEXT_MAP_INSERTED_SOURCE")
            if kind == "identity" and isinstance(source_span, dict):
                source_length = source_span.get("end_byte", 0) - source_span.get("start_byte", 0)
                if end - start != source_length:
                    rules.add("TEXT_MAP_IDENTITY_LENGTH")
        if boundary_info is not None and expected_start != boundary_info[0]:
            coverage_valid = False
        if not coverage_valid and ranges_in_bounds:
            rules.add("TEXT_MAP_COVERAGE")

    footnotes = [
        footnote
        for footnote in document.get("footnotes", [])
        if isinstance(footnote, dict)
    ] if isinstance(document, dict) else []
    footnote_ids = [footnote.get("footnote_id") for footnote in footnotes]
    if len(footnote_ids) != len(set(footnote_ids)):
        rules.add("DOC_DUPLICATE_FOOTNOTE_ID")
    elif all(isinstance(footnote_id, str) for footnote_id in footnote_ids) and (
        footnote_ids != sorted(footnote_ids, key=utf8_sort_key)
    ):
        rules.add("DOC_FOOTNOTE_ORDER")
    known_footnotes = set(footnote_ids)
    if any(
        value.get("kind") == "footnote_reference"
        and value.get("footnote_id") not in known_footnotes
        for value in document_objects
    ):
        rules.add("DOC_UNKNOWN_FOOTNOTE")

    anchor_definitions = [
        value.get("anchor_id")
        for value in document_objects
        if (
            value.get("kind") == "anchor"
            or (value.get("kind") == "heading" and value.get("anchor_id") is not None)
        )
    ]
    if len(anchor_definitions) != len(set(anchor_definitions)):
        rules.add("DOC_DUPLICATE_ANCHOR_ID")
    known_anchors = set(anchor_definitions)
    unknown_anchor = False
    for value in document_objects:
        if value.get("kind") == "reference" and value.get("target") not in known_anchors:
            unknown_anchor = True
        if value.get("kind") == "link":
            target = value.get("target", {})
            if (
                isinstance(target, dict)
                and target.get("kind") == "internal"
                and target.get("anchor_id") not in known_anchors
            ):
                unknown_anchor = True
    if unknown_anchor:
        rules.add("DOC_UNKNOWN_ANCHOR")

    resources = instance.get("resources", {})
    font_faces = resources.get("font_faces", []) if isinstance(resources, dict) else []
    images = resources.get("images", []) if isinstance(resources, dict) else []
    font_ids = [font.get("font_face_id") for font in font_faces if isinstance(font, dict)]
    font_families = [font.get("family") for font in font_faces if isinstance(font, dict)]
    image_ids = [image.get("image_id") for image in images if isinstance(image, dict)]
    if len(font_ids) != len(set(font_ids)):
        rules.add("DOC_DUPLICATE_FONT_ID")
    elif all(type(font_id) is int for font_id in font_ids) and font_ids != list(range(len(font_ids))):
        rules.add("DOC_FONT_INDEX")
    if len(image_ids) != len(set(image_ids)):
        rules.add("DOC_DUPLICATE_IMAGE_ID")
    elif all(type(image_id) is int for image_id in image_ids) and image_ids != list(range(len(image_ids))):
        rules.add("DOC_IMAGE_INDEX")
    if len(font_families) != len(set(font_families)):
        rules.add("DOC_DUPLICATE_FONT_FAMILY")
    if any(
        not isinstance(family, str)
        or not family
        or family.isspace()
        or any(ord(character) < 0x20 or ord(character) == 0x7F for character in family)
        for family in font_families
    ):
        rules.add("DOC_FONT_FAMILY_NAME")
    known_font_families = {
        family for family in font_families if isinstance(family, str)
    }
    known_images = set(image_ids)
    if any(
        value.get("kind") == "figure" and value.get("image_id") not in known_images
        for value in document_objects
    ):
        rules.add("DOC_UNKNOWN_IMAGE")

    page_masters = instance.get("page_masters", {})
    masters = page_masters.get("masters", []) if isinstance(page_masters, dict) else []
    master_ids = [master.get("master_id") for master in masters if isinstance(master, dict)]
    if len(master_ids) != len(set(master_ids)):
        rules.add("PAGE_MASTER_DUPLICATE_ID")
    elif all(isinstance(master_id, str) for master_id in master_ids) and (
        master_ids != sorted(master_ids, key=utf8_sort_key)
    ):
        rules.add("PAGE_MASTER_ORDER")
    known_masters = set(master_ids)
    if (
        isinstance(page_masters, dict)
        and page_masters.get("default_master_id") not in known_masters
    ):
        rules.add("PAGE_MASTER_DEFAULT_UNKNOWN")
    selection_rules = (
        page_masters.get("selection_rules", []) if isinstance(page_masters, dict) else []
    )
    if any(
        not isinstance(selection_rule, dict)
        or selection_rule.get("source_order") != index
        for index, selection_rule in enumerate(selection_rules)
    ):
        rules.add("PAGE_MASTER_SOURCE_ORDER")
    if any(
        isinstance(selection_rule, dict)
        and selection_rule.get("master_id") not in known_masters
        for selection_rule in selection_rules
    ):
        rules.add("PAGE_MASTER_RULE_UNKNOWN")
    for master in masters:
        if not isinstance(master, dict):
            continue
        width = master.get("width")
        height = master.get("height")
        if type(width) is not int or type(height) is not int:
            continue
        for region_name in ("body", "header", "footer", "footnote"):
            region = master.get(region_name)
            if not isinstance(region, dict):
                continue
            if all(type(region.get(key)) is int for key in ("x", "y", "width", "height")) and (
                region["x"] < 0
                or region["y"] < 0
                or region["x"] + region["width"] > width
                or region["y"] + region["height"] > height
            ):
                rules.add("PAGE_MASTER_RECT_BOUNDS")

    for table in (value for value in document_objects if value.get("kind") == "table"):
        columns = table.get("columns", [])
        head = table.get("head", [])
        body = table.get("body", [])
        if not all(isinstance(part, list) for part in (columns, head, body)):
            continue
        column_count = len(columns)
        rows = [*head, *body]
        occupied_rows = [0] * column_count
        grid_valid = column_count > 0
        crosses_head_body = False
        for row_index, row in enumerate(rows):
            if not isinstance(row, dict):
                grid_valid = False
                cells = []
            else:
                cells = row.get("cells", [])
            if not isinstance(cells, list):
                grid_valid = False
                cells = []
            for cell in cells:
                if not isinstance(cell, dict):
                    grid_valid = False
                    continue
                colspan = cell.get("colspan")
                rowspan = cell.get("rowspan")
                if (
                    type(colspan) is not int
                    or type(rowspan) is not int
                    or colspan <= 0
                    or rowspan <= 0
                ):
                    grid_valid = False
                    continue
                column_index = next(
                    (
                        index
                        for index, remaining in enumerate(occupied_rows)
                        if remaining == 0
                    ),
                    column_count,
                )
                column_end = column_index + colspan
                row_end = row_index + rowspan
                if (
                    column_end > column_count
                    or row_end > len(rows)
                    or any(
                        occupied_rows[target_column] != 0
                        for target_column in range(column_index, column_end)
                    )
                ):
                    grid_valid = False
                    continue
                if row_index < len(head) and row_end > len(head):
                    crosses_head_body = True
                for target_column in range(column_index, column_end):
                    occupied_rows[target_column] = rowspan
            if 0 in occupied_rows:
                grid_valid = False
            for column_index, remaining in enumerate(occupied_rows):
                if remaining > 0:
                    occupied_rows[column_index] = remaining - 1
        if any(remaining != 0 for remaining in occupied_rows):
            grid_valid = False
        if not grid_valid:
            rules.add("TABLE_GRID")
        if crosses_head_body:
            rules.add("TABLE_HEAD_BODY_CROSS")

    rules_in_order = instance.get("style_sheet", {}).get("rules", [])
    if isinstance(rules_in_order, list):
        if any(
            not isinstance(rule, dict) or rule.get("source_order") != index
            for index, rule in enumerate(rules_in_order)
        ):
            rules.add("STYLE_SOURCE_ORDER")

        style_rules = [rule for rule in rules_in_order if isinstance(rule, dict)]
        style_ids = [
            rule.get("style_id")
            for rule in style_rules
            if isinstance(rule.get("style_id"), str)
        ]
        if len(style_ids) != len(set(style_ids)):
            rules.add("STYLE_DUPLICATE_ID")
        known_style_ids = set(style_ids)

        extends_by_style: dict[str, str] = {}
        for rule in style_rules:
            selector = rule.get("selector")
            if not isinstance(selector, str) or STYLE_SELECTOR.fullmatch(selector) is None:
                rules.add("STYLE_SELECTOR_SYNTAX")
            else:
                class_components = selector.split(".")[1:]
                if len(class_components) != len(set(class_components)):
                    rules.add("STYLE_SELECTOR_DUPLICATE_CLASS")
                elif class_components != sorted(class_components, key=utf8_sort_key):
                    rules.add("STYLE_SELECTOR_CLASS_ORDER")

            for declaration in rule.get("declarations", []):
                if not isinstance(declaration, dict):
                    continue
                declaration_name = declaration.get("name")
                if not isinstance(declaration_name, str) or declaration_name not in STYLE_PROPERTY_NAMES:
                    rules.add("STYLE_DECLARATION_NAME")
                    continue
                declaration_value = declaration.get("value", {})
                property_type_valid = False
                if declaration_name == "font_family":
                    families = (
                        declaration_value.get("families")
                        if isinstance(declaration_value, dict)
                        and declaration_value.get("kind") == "font_family_list"
                        else None
                    )
                    property_type_valid = isinstance(families, list) and bool(families)
                    if property_type_valid:
                        aliases_valid = all(
                            isinstance(family, str)
                            and bool(family)
                            and not family.isspace()
                            and not any(
                                ord(character) < 0x20 or ord(character) == 0x7F
                                for character in family
                            )
                            for family in families
                        )
                        if not aliases_valid or len(families) != len(set(families)):
                            rules.add("STYLE_FONT_FAMILY_LIST")
                        elif not any(family in known_font_families for family in families):
                            rules.add("STYLE_UNKNOWN_FONT_FAMILY")
                elif declaration_name in {"font_size", "line_height"}:
                    property_type_valid = (
                        isinstance(declaration_value, dict)
                        and declaration_value.get("kind") == "length"
                        and type(declaration_value.get("value")) is int
                        and 1 <= declaration_value["value"] <= JSON_SAFE_INTEGER_MAX
                    )
                if declaration_name == "page":
                    page_value_valid = isinstance(declaration_value, dict) and (
                        (
                            declaration_value.get("kind") == "keyword"
                            and declaration_value.get("value") == "auto"
                        )
                        or (
                            declaration_value.get("kind") == "string"
                            and isinstance(declaration_value.get("value"), str)
                            and CLASS_TOKEN.fullmatch(declaration_value["value"]) is not None
                        )
                    )
                    if not page_value_valid:
                        rules.add("STYLE_PAGE_VALUE")
                    property_type_valid = page_value_valid
                if not property_type_valid and declaration_name != "page":
                    rules.add("STYLE_PROPERTY_TYPE")

            style_id = rule.get("style_id")
            parent_style_id = rule.get("extends")
            if isinstance(parent_style_id, str):
                if parent_style_id not in known_style_ids:
                    rules.add("STYLE_EXTENDS_UNKNOWN")
                elif isinstance(style_id, str):
                    extends_by_style[style_id] = parent_style_id

        visit_state: dict[str, int] = {}
        extends_cycle = False
        for start in known_style_ids:
            if visit_state.get(start) == 2:
                continue
            path: list[str] = []
            current: str | None = start
            while current is not None:
                state = visit_state.get(current, 0)
                if state == 1:
                    extends_cycle = True
                    break
                if state == 2:
                    break
                visit_state[current] = 1
                path.append(current)
                current = extends_by_style.get(current)
            for style_id in path:
                visit_state[style_id] = 2
            if extends_cycle:
                break

        if extends_cycle:
            rules.add("STYLE_EXTENDS_CYCLE")

        style_prerequisite_rules = {
            "STYLE_CLASS_TOKEN",
            "STYLE_CLASS_ORDER",
            "STYLE_SELECTOR_SYNTAX",
            "STYLE_SELECTOR_DUPLICATE_CLASS",
            "STYLE_SELECTOR_CLASS_ORDER",
            "STYLE_DUPLICATE_ID",
            "STYLE_EXTENDS_UNKNOWN",
            "STYLE_EXTENDS_CYCLE",
            "STYLE_DECLARATION_NAME",
            "STYLE_PROPERTY_TYPE",
            "STYLE_FONT_FAMILY_LIST",
            "STYLE_UNKNOWN_FONT_FAMILY",
            "STYLE_PAGE_VALUE",
        }
        if not rules.intersection(style_prerequisite_rules):
            rules_by_id = {rule["style_id"]: rule for rule in style_rules}

            def expanded_chain(rule: dict[str, Any]) -> list[dict[str, Any]]:
                chain: list[dict[str, Any]] = []
                current: dict[str, Any] | None = rule
                while current is not None:
                    chain.append(current)
                    parent = current.get("extends")
                    current = rules_by_id.get(parent) if isinstance(parent, str) else None
                chain.reverse()
                return chain

            for block in (
                value
                for value in document_objects
                if value.get("kind") in {"paragraph", "heading", "list", "table", "figure"}
                and (
                    value.get("kind") == "list"
                    or any(
                        child.get("kind") in {"text", "reference", "footnote_reference"}
                        for child in walk_objects(value)
                    )
                )
            ):
                block_kind = block.get("kind")
                block_classes = set(block.get("classes", []))
                winners: dict[str, tuple[tuple[int, ...], Any]] = {}
                for matched_rule in style_rules:
                    selector = matched_rule["selector"]
                    selector_parts = selector.split(".")
                    selector_classes = selector_parts[1:]
                    if selector_parts[0] != block_kind or not set(selector_classes) <= block_classes:
                        continue
                    specificity = (0, len(selector_classes), 1)
                    for inheritance_depth, origin_rule in enumerate(expanded_chain(matched_rule)):
                        for declaration_order, declaration in enumerate(
                            origin_rule.get("declarations", [])
                        ):
                            name = declaration.get("name")
                            if name not in STYLE_PROPERTY_NAMES:
                                continue
                            precedence = (
                                int(declaration.get("important") is True),
                                *specificity,
                                matched_rule["source_order"],
                                inheritance_depth,
                                declaration_order,
                            )
                            current = winners.get(name)
                            if current is None or precedence > current[0]:
                                winners[name] = (precedence, declaration.get("value"))
                if not {"font_family", "font_size", "line_height"} <= set(winners):
                    rules.add("STYLE_REQUIRED_COMPUTED_VALUE")

    for value in document_objects:
        if value.get("kind") != "link":
            continue
        target = value.get("target", {})
        if not isinstance(target, dict) or target.get("kind") != "uri":
            continue
        uri = target.get("uri")
        scheme = uri.split(":", 1)[0] if isinstance(uri, str) and ":" in uri else None
        if (
            scheme not in {"http", "https", "mailto", "tel"}
            or (allowed_uri_schemes is not None and scheme not in allowed_uri_schemes)
        ):
            rules.add("DOC_URI_SCHEME")
        if isinstance(uri, str) and max_uri_bytes is not None:
            try:
                uri_length = len(uri.encode("utf-8"))
            except UnicodeEncodeError:
                uri_length = max_uri_bytes + 1
            if uri_length > max_uri_bytes:
                rules.add("DOC_URI_LIMIT")

    paths: list[Any] = [source.get("uri") for source in source_records]
    paths.extend(font.get("uri") for font in font_faces if isinstance(font, dict))
    paths.extend(image.get("uri") for image in images if isinstance(image, dict))
    if any(not is_portable_path(path) for path in paths):
        rules.add("PATH_PORTABILITY")
    if rules.intersection({"TABLE_GRID", "TABLE_HEAD_BODY_CROSS"}):
        rules.discard("DOC_NODE_INDEX")
    if "STYLE_REQUIRED_COMPUTED_VALUE" in rules and len(rules) > 1:
        rules.discard("STYLE_REQUIRED_COMPUTED_VALUE")
    return rules


def manifest_rule_ids(instance: dict[str, Any]) -> set[str]:
    rules: set[str] = set()
    if instance.get("data_versions") != {
        "unicode": "16.0.0",
        "japanese_line_break": "typaxis-jlreq-horizontal/1.0.0",
        "shaper_backend": "typaxis-reference-shaper",
        "shaper_version": "0.1.0",
    }:
        rules.add("MANIFEST_DATA_VERSION")
    if instance.get("status") not in {"built", "failed"}:
        rules.add("MANIFEST_STATUS")
    if instance.get("engine", {}).get("name") != "typaxis":
        rules.add("MANIFEST_ENGINE")
    input_profile = instance.get("input_profile")
    package_input = instance.get("package_input")
    status = instance.get("status")
    if input_profile == "typaxis.reference-source/1" and package_input is not None:
        rules.add("MANIFEST_INPUT_PROFILE")
    elif input_profile == "typaxis.machine-pdf/paragraph-1":
        if status == "built" and (
            not isinstance(package_input, dict)
            or package_input.get("contract") is None
            or package_input.get("canonical_sha256") is None
        ):
            rules.add("MANIFEST_INPUT_PROFILE")
        elif isinstance(package_input, dict) and (
            (package_input.get("contract") is None)
            != (package_input.get("canonical_sha256") is None)
        ):
            rules.add("MANIFEST_INPUT_PROFILE")
    collections = (
        (instance.get("inputs", []), lambda record: record.get("uri")),
        (instance.get("fonts", []), lambda record: record.get("font_face_id")),
        (instance.get("images", []), lambda record: record.get("image_id")),
    )
    for records, identity in collections:
        identities = [identity(record) for record in records if isinstance(record, dict)]
        if len(identities) != len(set(identities)):
            rules.add("MANIFEST_DUPLICATE_RESOURCE")
        if identities != sorted(identities):
            rules.add("MANIFEST_ORDER")
    for record in instance.get("fonts", []):
        if not isinstance(record, dict):
            continue
        units_per_em = record.get("units_per_em")
        glyph_count = record.get("glyph_count")
        if type(units_per_em) is int and not 16 <= units_per_em <= 16384:
            rules.add("MANIFEST_FONT_UNITS_RANGE")
        if type(glyph_count) is int and not 1 <= glyph_count <= 4294967295:
            rules.add("MANIFEST_GLYPH_COUNT_RANGE")
    for record in instance.get("images", []):
        if not isinstance(record, dict):
            continue
        width = record.get("pixel_width")
        height = record.get("pixel_height")
        if type(width) is int and not 1 <= width <= 4294967295:
            rules.add("MANIFEST_IMAGE_WIDTH_RANGE")
        if type(height) is int and not 1 <= height <= 4294967295:
            rules.add("MANIFEST_IMAGE_HEIGHT_RANGE")

    layout = instance.get("layout")
    if isinstance(layout, dict):
        pass_count = layout.get("pass_count")
        selected_state = layout.get("selected_state")
        if type(pass_count) is int and type(selected_state) is int:
            if not 1 <= selected_state <= pass_count:
                rules.add("MANIFEST_SELECTED_STATE_RANGE")
            elif layout.get("status") == "converged" and selected_state != pass_count:
                rules.add("MANIFEST_CONVERGED_STATE")
    return rules


def config_rule_ids(instance: dict[str, Any]) -> set[str]:
    rules: set[str] = set()
    if instance.get("data_versions") != {
        "unicode": "16.0.0",
        "japanese_line_break": "typaxis-jlreq-horizontal/1.0.0",
    }:
        rules.add("CONFIG_DATA_VERSION")
    if instance.get("deterministic") is not True:
        rules.add("CONFIG_DETERMINISTIC")

    allowed_uri_schemes = instance.get("allowed_uri_schemes")
    if isinstance(allowed_uri_schemes, list) and not is_canonical_string_list(
        allowed_uri_schemes
    ):
        rules.add("CONFIG_URI_SCHEME_ORDER")
    resource_roots = instance.get("resource_roots")
    if isinstance(resource_roots, list) and not is_canonical_string_list(resource_roots):
        rules.add("CONFIG_RESOURCE_ROOT_ORDER")

    limits = instance.get("limits", {})
    if isinstance(limits, dict):
        max_document_package_bytes = limits.get("max_document_package_bytes")
        if (
            type(max_document_package_bytes) is int
            and max_document_package_bytes > MAX_DOCUMENT_PACKAGE_BYTES
        ):
            rules.add("CONFIG_DOCUMENT_PACKAGE_BYTES")
        max_json_nesting_depth = limits.get("max_json_nesting_depth")
        if (
            type(max_json_nesting_depth) is int
            and max_json_nesting_depth > MAX_JSON_NESTING_DEPTH
        ):
            rules.add("CONFIG_JSON_NESTING_DEPTH")
        max_ast_nesting_depth = limits.get("max_ast_nesting_depth")
        if (
            type(max_ast_nesting_depth) is int
            and max_ast_nesting_depth > MAX_AST_NESTING_DEPTH
        ):
            rules.add("CONFIG_AST_NESTING_DEPTH")
        max_fonts = limits.get("max_fonts")
        if type(max_fonts) is int and max_fonts > MAX_FONT_SUBSET_TAGS:
            rules.add("CONFIG_FONT_COUNT_PROFILE_MAX")
        max_source_bytes = limits.get("max_source_bytes")
        max_input_bytes = limits.get("max_input_bytes")
        if (
            type(max_source_bytes) is int
            and type(max_input_bytes) is int
            and max_source_bytes > max_input_bytes
        ):
            rules.add("CONFIG_SOURCE_INPUT_LIMIT")

        max_text_buffer_bytes = limits.get("max_text_buffer_bytes")
        max_text_bytes = limits.get("max_text_bytes")
        if (
            type(max_text_buffer_bytes) is int
            and type(max_text_bytes) is int
            and max_text_buffer_bytes > max_text_bytes
        ):
            rules.add("CONFIG_TEXT_BUFFER_LIMIT")

        max_shaping_context_bytes = limits.get("max_shaping_context_bytes")
        if (
            type(max_shaping_context_bytes) is int
            and type(max_text_buffer_bytes) is int
            and max_shaping_context_bytes > max_text_buffer_bytes
        ):
            rules.add("CONFIG_SHAPING_CONTEXT_LIMIT")

        max_resource_bytes = limits.get("max_resource_bytes")
        max_font_bytes = limits.get("max_font_bytes")
        if (
            type(max_font_bytes) is int
            and type(max_resource_bytes) is int
            and max_font_bytes > max_resource_bytes
        ):
            rules.add("CONFIG_FONT_RESOURCE_LIMIT")
        max_image_bytes = limits.get("max_image_bytes")
        if (
            type(max_image_bytes) is int
            and type(max_resource_bytes) is int
            and max_image_bytes > max_resource_bytes
        ):
            rules.add("CONFIG_IMAGE_RESOURCE_LIMIT")
    return rules


def capabilities_rule_ids(instance: dict[str, Any]) -> set[str]:
    limits = instance.get("machine_input", {}).get("limits", {})
    if limits != {
        "max_document_package_bytes": {
            "default": 134_217_728,
            "maximum": MAX_DOCUMENT_PACKAGE_BYTES,
        },
        "max_json_nesting_depth": {
            "default": 256,
            "maximum": MAX_JSON_NESTING_DEPTH,
        },
    }:
        return {"CAPABILITY_LIMIT_DESCRIPTOR"}
    return set()


def flow_position_key(position: Any) -> tuple[Any, ...] | None:
    if not isinstance(position, dict):
        return None
    path = position.get("block_child_path")
    if not isinstance(path, list) or not all(type(item) is int for item in path):
        return None
    fields = (
        position.get("global_flow_ordinal"),
        position.get("owner"),
        tuple(path),
        position.get("owner_local_boundary"),
    )
    return fields if type(fields[0]) is int and type(fields[1]) is int and type(fields[3]) is int else None


def generated_key_tuple(key: Any) -> tuple[Any, ...] | None:
    if not isinstance(key, dict):
        return None
    owner = key.get("owner")
    generation_kind = key.get("generation_kind")
    ordinal = key.get("owner_local_ordinal")
    if type(owner) is not int or generation_kind not in GENERATION_KIND_ORDER or type(ordinal) is not int:
        return None
    return owner, GENERATION_KIND_ORDER[generation_kind], ordinal


def canonical_generated_records_from_state(state: Any) -> list[dict[str, Any]]:
    if not isinstance(state, dict):
        return []
    return copy.deepcopy(state.get("resolved_generated_text", []))


def reference_fingerprint(records: Any) -> str:
    return hashlib.sha256(
        jcs_bytes({
            "algorithm": "typaxis.reference-state.jcs-sha256/1",
            "resolved_generated_text": records,
        })
    ).hexdigest()


def generated_record_rule_ids(records: Any, order_rule: str, span_rule: str) -> set[str]:
    rules: set[str] = set()
    if not isinstance(records, list):
        return rules
    keys: list[tuple[Any, ...]] = []
    for record in records:
        if not isinstance(record, dict):
            continue
        key = generated_key_tuple(record.get("key", record.get("buffer_key")))
        start = record.get("start_byte")
        end = record.get("end_byte")
        utf8 = record.get("utf8")
        if key is not None and type(start) is int and type(end) is int:
            keys.append((*key, start, end))
        if (
            type(start) is int and type(end) is int and isinstance(utf8, str)
            and (start > end or end - start != len(utf8.encode("utf-8")))
        ):
            rules.add(span_rule)
    if len(keys) != len(set(keys)) or keys != sorted(keys):
        rules.add(order_rule)
    sites = sorted({key[:3] for key in keys})
    by_owner_kind: dict[tuple[Any, Any], list[int]] = {}
    for owner, generation_kind, ordinal in sites:
        by_owner_kind.setdefault((owner, generation_kind), []).append(ordinal)
    if any(ordinals != list(range(len(ordinals))) for ordinals in by_owner_kind.values()):
        rules.add(order_rule)
    return rules


def flow_registry_info(record: Any) -> tuple[set[bytes], set[int], set[str]]:
    """Validate and index the canonical FlowTree boundary registry in one state."""

    rules: set[str] = set()
    if not isinstance(record, dict):
        return set(), set(), rules
    epoch = record.get("layout_epoch")
    positions = record.get("flow_positions")
    if not isinstance(positions, list) or not positions:
        return set(), set(), {"TRACE_FLOW_REGISTRY"}
    encoded: set[bytes] = set()
    owners: set[int] = set()
    boundaries: set[tuple[Any, ...]] = set()
    boundary_keys: list[tuple[Any, ...]] = []
    for ordinal, position in enumerate(positions):
        key = flow_position_key(position)
        if (
            key is None
            or position.get("global_flow_ordinal") != ordinal
            or position.get("epoch") != epoch
        ):
            rules.add("TRACE_FLOW_REGISTRY")
            continue
        boundary = (position.get("owner"), tuple(position.get("block_child_path", [])), position.get("owner_local_boundary"))
        boundary_keys.append(boundary)
        if boundary in boundaries:
            rules.add("TRACE_FLOW_REGISTRY")
        boundaries.add(boundary)
        encoded.add(jcs_bytes(position))
        owners.add(position["owner"])
    first = positions[0]
    if (
        not isinstance(first, dict)
        or first.get("global_flow_ordinal") != 0
        or first.get("owner") != 0
        or first.get("block_child_path") != []
        or first.get("owner_local_boundary") != 0
    ):
        rules.add("TRACE_FLOW_REGISTRY")
    if len(encoded) != len(positions):
        rules.add("TRACE_FLOW_REGISTRY")
    if len(positions) == 2:
        rules.add("TRACE_FLOW_REGISTRY")
    intermediate_keys = boundary_keys[1:-1] if len(boundary_keys) > 1 else []
    if intermediate_keys != sorted(intermediate_keys):
        rules.add("TRACE_FLOW_REGISTRY")
    if len(positions) > 1:
        terminal = positions[-1]
        if (
            not isinstance(terminal, dict)
            or terminal.get("owner") != 0
            or terminal.get("block_child_path") != []
            or terminal.get("owner_local_boundary") != 1
        ):
            rules.add("TRACE_FLOW_REGISTRY")
    local_boundaries: dict[tuple[Any, ...], list[int]] = {}
    for owner, path, boundary in intermediate_keys:
        if type(boundary) is int:
            local_boundaries.setdefault((owner, path), []).append(boundary)
    if any(
        values != list(range(len(values)))
        for values in local_boundaries.values()
    ):
        rules.add("TRACE_FLOW_REGISTRY")
    return encoded, owners, rules


def trace_record_rule_ids(record: Any, expected_resolved_input: str) -> set[str]:
    rules: set[str] = set()
    if not isinstance(record, dict):
        return rules
    epoch = record.get("layout_epoch")
    flow_positions, flow_owners, flow_rules = flow_registry_info(record)
    rules.update(flow_rules)
    if isinstance(epoch, dict) and epoch.get("resolved_input_sha256") != expected_resolved_input:
        rules.add("TRACE_EPOCH_INPUT")
    rules.update(
        generated_record_rule_ids(
            canonical_generated_records_from_state(record),
            "TRACE_GENERATED_TEXT_ORDER",
            "TRACE_GENERATED_TEXT_SPAN",
        )
    )
    pages = record.get("pages", [])
    if isinstance(pages, list):
        if not pages:
            rules.add("TRACE_EMPTY_PAGES")
        elif any(
            not isinstance(page, dict) or page.get("page_index") != page_index
            for page_index, page in enumerate(pages)
        ):
            rules.add("TRACE_PAGE_INDEX")
    known_pages = {
        page.get("page_index") for page in pages if isinstance(page, dict)
    }
    frames_by_page_key: dict[tuple[Any, Any, Any], dict[str, Any]] = {}
    flattened_fragments: list[dict[str, Any]] = []

    def rect_within(child: Any, parent: Any) -> bool:
        if not isinstance(child, dict) or not isinstance(parent, dict):
            return False
        if not all(type(rect.get(field)) is int for rect in (child, parent) for field in ("x", "y", "width", "height")):
            return False
        return (
            child["x"] >= parent["x"] and child["y"] >= parent["y"]
            and child["x"] + child["width"] <= parent["x"] + parent["width"]
            and child["y"] + child["height"] <= parent["y"] + parent["height"]
        )
    for page in pages if isinstance(pages, list) else []:
        if not isinstance(page, dict):
            continue
        frames = page.get("frames", [])
        frame_keys = [
            (FRAME_KIND_ORDER.get(frame.get("kind"), 99), frame.get("column_index"))
            for frame in frames
            if isinstance(frame, dict)
        ]
        frame_by_key = {
            (frame.get("kind"), frame.get("column_index")): frame
            for frame in frames if isinstance(frame, dict)
        }
        for (kind, column_index), frame in frame_by_key.items():
            frames_by_page_key[(page.get("page_index"), kind, column_index)] = frame
        dense_frame_columns = True
        previous: tuple[int, int] | None = None
        for frame_key in frame_keys:
            expected_column = previous[1] + 1 if previous is not None and previous[0] == frame_key[0] else 0
            if frame_key[1] != expected_column:
                dense_frame_columns = False
            previous = frame_key
        if (
            not frame_keys or frame_keys[0] != (FRAME_KIND_ORDER["body"], 0)
            or len(frame_keys) != len(set(frame_keys))
            or frame_keys != sorted(frame_keys)
            or not dense_frame_columns
        ):
            rules.add("TRACE_FRAME_ORDER")
        footnote_ids = page.get("footnote_ids", [])
        if isinstance(footnote_ids, list) and not is_canonical_string_list(footnote_ids):
            rules.add("TRACE_FOOTNOTE_ORDER")
        fragments = page.get("fragments", [])
        fragment_keys: list[tuple[Any, ...]] = []
        for fragment in fragments if isinstance(fragments, list) else []:
            if not isinstance(fragment, dict):
                continue
            flattened_fragments.append(fragment)
            start = fragment.get("start")
            end = fragment.get("end")
            start_key = flow_position_key(start)
            end_key = flow_position_key(end)
            owner = fragment.get("owner")
            frame = frame_by_key.get((fragment.get("frame_kind"), fragment.get("column_index")))
            if frame is None or not rect_within(fragment.get("bounds"), frame.get("bounds")):
                rules.add("TRACE_FRAGMENT_FRAME")
            if (
                start_key is None or end_key is None or start_key >= end_key
                or not isinstance(start, dict) or not isinstance(end, dict)
                or start.get("epoch") != epoch or end.get("epoch") != epoch
                or start.get("owner") != owner
                or jcs_bytes(start) not in flow_positions
                or jcs_bytes(end) not in flow_positions
            ):
                rules.add("TRACE_FLOW_POSITION")
                continue
            fragment_keys.append((start_key, owner, fragment.get("owner_local_ordinal")))
        if len(fragment_keys) != len(set(fragment_keys)) or fragment_keys != sorted(fragment_keys):
            rules.add("TRACE_FRAGMENT_ORDER")

        floats = page.get("float_decisions", [])
        if isinstance(floats, list):
            float_keys = [
                (decision.get("owner"), decision.get("owner_local_ordinal"))
                for decision in floats if isinstance(decision, dict)
            ]
            if len(float_keys) != len(set(float_keys)) or float_keys != sorted(float_keys):
                rules.add("TRACE_FLOAT_ORDER")
            if any(
                frame_by_key.get((decision.get("frame_kind"), decision.get("column_index"))) is None
                or not rect_within(
                    decision.get("bounds"),
                    frame_by_key[(decision.get("frame_kind"), decision.get("column_index"))].get("bounds"),
                )
                for decision in floats if isinstance(decision, dict)
            ):
                rules.add("TRACE_FLOAT_FRAME")
            if any(
                decision.get("owner") not in flow_owners
                for decision in floats if isinstance(decision, dict)
            ):
                rules.add("TRACE_FLOW_REGISTRY")

        columns = page.get("column_decisions", [])
        if isinstance(columns, list):
            column_keys = [
                (decision.get("container"), decision.get("column_index"))
                for decision in columns if isinstance(decision, dict)
            ]
            dense_columns = True
            previous_column: tuple[Any, Any] | None = None
            for key in column_keys:
                expected_column = previous_column[1] + 1 if previous_column is not None and previous_column[0] == key[0] else 0
                if key[1] != expected_column:
                    dense_columns = False
                previous_column = key
            if len(column_keys) != len(set(column_keys)) or column_keys != sorted(column_keys) or not dense_columns:
                rules.add("TRACE_COLUMN_ORDER")
            if any(
                decision.get("container") not in flow_owners
                for decision in columns if isinstance(decision, dict)
            ):
                rules.add("TRACE_FLOW_REGISTRY")

        references = page.get("resolved_references", [])
        reference_rules = generated_record_rule_ids(
            references, "TRACE_RESOLVED_REFERENCE_ORDER", "TRACE_RESOLVED_REFERENCE_SPAN"
        )
        if isinstance(references, list):
            reference_sort_keys = []
            for reference in references:
                if not isinstance(reference, dict):
                    continue
                key = generated_key_tuple(reference.get("buffer_key"))
                if key is not None:
                    reference_sort_keys.append((*key, reference.get("start_byte"), reference.get("end_byte"), utf8_sort_key(reference.get("anchor_id", ""))))
            if len(reference_sort_keys) != len(set(reference_sort_keys)) or reference_sort_keys != sorted(reference_sort_keys):
                reference_rules.add("TRACE_RESOLVED_REFERENCE_ORDER")
        rules.update(reference_rules)

    placed_anchors = record.get("placed_anchors", [])
    if isinstance(placed_anchors, list):
        anchor_ids = [
            anchor.get("anchor_id")
            for anchor in placed_anchors if isinstance(anchor, dict)
        ]
        if (
            len(anchor_ids) != len(placed_anchors)
            or len(anchor_ids) != len(set(anchor_ids))
            or not all(isinstance(anchor_id, str) for anchor_id in anchor_ids)
            or anchor_ids != sorted(anchor_ids, key=utf8_sort_key)
        ):
            rules.add("TRACE_PLACED_ANCHOR")
        for anchor in placed_anchors:
            if not isinstance(anchor, dict):
                continue
            frame = frames_by_page_key.get((
                anchor.get("page_index"), anchor.get("frame_kind"), anchor.get("column_index")
            ))
            point = anchor.get("position_in_frame")
            bounds = frame.get("bounds") if isinstance(frame, dict) else None
            if (
                not isinstance(point, dict)
                or not isinstance(bounds, dict)
                or not all(type(point.get(axis)) is int for axis in ("x", "y"))
                or not all(
                    type(bounds.get(field)) is int
                    for field in ("x", "y", "width", "height")
                )
                or point["x"] < 0
                or point["y"] < 0
                or point["x"] > bounds["width"]
                or point["y"] > bounds["height"]
                or not -JSON_SAFE_INTEGER_MAX <= bounds.get("x", 0) + point["x"] <= JSON_SAFE_INTEGER_MAX
                or not -JSON_SAFE_INTEGER_MAX <= bounds.get("y", 0) + point["y"] <= JSON_SAFE_INTEGER_MAX
            ):
                rules.add("TRACE_PLACED_ANCHOR")

    positions = record.get("flow_positions", [])
    if isinstance(positions, list) and positions:
        if len(positions) == 1:
            if flattened_fragments and "TRACE_FLOW_POSITION" not in rules:
                rules.add("TRACE_FRAGMENT_CLOSURE")
        elif not rules.intersection({
            "TRACE_FLOW_REGISTRY", "TRACE_FLOW_POSITION", "TRACE_FRAGMENT_ORDER",
        }):
            fragment_positions = [
                (jcs_bytes(fragment.get("start")), jcs_bytes(fragment.get("end")))
                for fragment in flattened_fragments
            ]
            expected_start = jcs_bytes(positions[1])
            expected_end = jcs_bytes(positions[-1])
            if (
                not fragment_positions
                or fragment_positions[0][0] != expected_start
                or fragment_positions[-1][1] != expected_end
                or any(
                    previous[1] != following[0]
                    for previous, following in zip(fragment_positions, fragment_positions[1:])
                )
            ):
                rules.add("TRACE_FRAGMENT_CLOSURE")
        ordinals_by_owner: dict[Any, list[Any]] = {}
        for fragment in flattened_fragments:
            ordinals_by_owner.setdefault(fragment.get("owner"), []).append(
                fragment.get("owner_local_ordinal")
            )
        if any(
            not all(type(value) is int for value in values)
            or sorted(values) != list(range(len(values)))
            for values in ordinals_by_owner.values()
        ):
            rules.add("TRACE_FRAGMENT_ORDINAL")
    return rules


def trace_rule_ids(instance: dict[str, Any]) -> set[str]:
    rules: set[str] = set()
    passes = instance.get("passes", [])
    result = instance.get("result", {})
    initial_state = instance.get("initial_state", {})
    initial = instance.get("initial_fingerprint")
    initial_records = canonical_generated_records_from_state(initial_state)
    _, _, initial_flow_rules = flow_registry_info(initial_state)
    rules.update(initial_flow_rules)
    expected_initial_reference = reference_fingerprint(initial_records)
    initial_record_rules = generated_record_rule_ids(
        initial_records, "TRACE_GENERATED_TEXT_ORDER", "TRACE_GENERATED_TEXT_SPAN"
    )
    if (
        isinstance(initial_state, dict)
        and initial_state.get("layout_epoch", {}).get("resolved_input_sha256")
        != expected_initial_reference
    ):
        initial_record_rules.add("TRACE_EPOCH_INPUT")
    rules.update(initial_record_rules)
    if not initial_record_rules and isinstance(initial_state, dict):
        if hashlib.sha256(jcs_bytes(initial_state)).hexdigest() != initial:
            rules.add("TRACE_INITIAL_FINGERPRINT")

    record_rules: set[str] = set()
    if isinstance(passes, list):
        if any(
            not isinstance(layout_pass, dict)
            or layout_pass.get("pass_index") != pass_index
            for pass_index, layout_pass in enumerate(passes)
        ):
            rules.add("TRACE_PASS_INDEX")
        previous_state = initial_state
        for layout_pass in passes:
            if not isinstance(layout_pass, dict):
                continue
            expected_resolved_input = reference_fingerprint(
                canonical_generated_records_from_state(previous_state)
            )
            state = layout_pass.get("state", {})
            state_rules = trace_record_rule_ids(state, expected_resolved_input)
            record_rules.update(state_rules)
            rules.update(state_rules)
            if not state_rules and isinstance(state, dict) and (
                hashlib.sha256(jcs_bytes(state)).hexdigest()
                != layout_pass.get("output_fingerprint")
            ):
                rules.add("TRACE_FINGERPRINT")
            previous_state = state

            cost = layout_pass.get("cost", {})
            cost_components = (
                "keep", "widow_orphan", "heading_isolation", "table_split",
                "footnote_split", "unused_space", "overflow",
            )
            if isinstance(cost, dict):
                values = [cost.get(component) for component in cost_components]
                if all(type(value) is int for value in values) and type(cost.get("total")) is int:
                    total = sum(values)
                    if not -JSON_SAFE_INTEGER_MAX <= total <= JSON_SAFE_INTEGER_MAX or cost["total"] != total:
                        rules.add("TRACE_COST_TOTAL")

    pass_limit_invalid = (
        not isinstance(passes, list)
        or len(passes) > instance.get("max_layout_passes", 0)
        or (isinstance(result, dict) and result.get("pass_count") != len(passes))
    )
    if pass_limit_invalid:
        rules.add("TRACE_PASS_LIMIT")

    hash_prerequisites = {
        "TRACE_INITIAL_FINGERPRINT", "TRACE_FINGERPRINT", "TRACE_EPOCH_INPUT",
        "TRACE_PAGE_INDEX", "TRACE_EMPTY_PAGES", "TRACE_FRAME_ORDER",
        "TRACE_FOOTNOTE_ORDER", "TRACE_FLOW_POSITION", "TRACE_FRAGMENT_ORDER",
        "TRACE_FRAGMENT_FRAME", "TRACE_FLOAT_ORDER", "TRACE_FLOAT_FRAME",
        "TRACE_COLUMN_ORDER", "TRACE_GENERATED_TEXT_ORDER",
        "TRACE_GENERATED_TEXT_SPAN", "TRACE_RESOLVED_REFERENCE_ORDER",
        "TRACE_RESOLVED_REFERENCE_SPAN",
        "TRACE_FLOW_REGISTRY", "TRACE_PLACED_ANCHOR",
        "TRACE_FRAGMENT_CLOSURE", "TRACE_FRAGMENT_ORDINAL",
    }
    chain_invalid = False
    if passes and not rules.intersection(hash_prerequisites):
        expected_inputs = [initial, *(layout_pass.get("output_fingerprint") for layout_pass in passes[:-1])]
        chain_invalid = any(
            layout_pass.get("input_fingerprint") != expected
            for layout_pass, expected in zip(passes, expected_inputs)
        )
        if chain_invalid:
            rules.add("TRACE_CHAIN_BREAK")

    status = result.get("status") if isinstance(result, dict) else None
    max_layout_passes = instance.get("max_layout_passes")
    if status == "max_pass_fallback" and type(max_layout_passes) is int and len(passes) != max_layout_passes:
        rules.add("TRACE_MAX_PASS")

    if not pass_limit_invalid and not chain_invalid and not rules.intersection(hash_prerequisites):
        terminal_kinds: list[str | None] = []
        prior_state_fingerprints: list[Any] = []
        for layout_pass in passes:
            input_fingerprint = layout_pass.get("input_fingerprint")
            output_fingerprint = layout_pass.get("output_fingerprint")
            if input_fingerprint == output_fingerprint:
                terminal_kinds.append("stable")
            elif output_fingerprint in prior_state_fingerprints:
                terminal_kinds.append("cycle")
            else:
                terminal_kinds.append(None)
            prior_state_fingerprints.append(output_fingerprint)

        if any(kind is not None for kind in terminal_kinds[:-1]):
            rules.add("TRACE_TERMINATION")
        if terminal_kinds:
            final_kind = terminal_kinds[-1]
            expected_kind = {
                "converged": "stable", "cycle_fallback": "cycle", "max_pass_fallback": None,
            }.get(status)
            if status in {"cycle_fallback", "max_pass_fallback"} and final_kind != expected_kind:
                rules.add("TRACE_TERMINATION")

        if status == "converged":
            if not passes or passes[-1].get("input_fingerprint") != passes[-1].get("output_fingerprint"):
                rules.add("TRACE_FALSE_CONVERGENCE")
            if result.get("selected_state") != len(passes) or (
                passes and result.get("final_fingerprint") != passes[-1].get("output_fingerprint")
            ):
                rules.add("TRACE_SELECTED_STATE")
        elif status in {"cycle_fallback", "max_pass_fallback"} and passes:
            candidates = []
            for state_index, layout_pass in enumerate(passes, start=1):
                cost = layout_pass.get("cost", {})
                page_count = len(layout_pass.get("state", {}).get("pages", []))
                candidates.append(((cost.get("hard_violations", JSON_SAFE_INTEGER_MAX), cost.get("total", JSON_SAFE_INTEGER_MAX), page_count, state_index), state_index))
            selected_state = min(candidates)[1]
            if result.get("selected_state") != selected_state or result.get("final_fingerprint") != passes[selected_state - 1].get("output_fingerprint"):
                rules.add("TRACE_SELECTED_STATE")

        if status == "cycle_fallback" and passes:
            prior_states = [layout_pass.get("output_fingerprint") for layout_pass in passes[:-1]]
            repeated = passes[-1].get("output_fingerprint")
            cycle_start = next((index for index, fingerprint in enumerate(prior_states, start=1) if fingerprint == repeated), None)
            if cycle_start is None or result.get("cycle_start_state") != cycle_start:
                rules.add("TRACE_CYCLE_STATE")
    return rules


def cross_artifact_rule_ids(
    config: dict[str, Any],
    document: dict[str, Any],
    display: dict[str, Any],
    trace: dict[str, Any],
    manifest: dict[str, Any],
    artifact_directory: Path,
) -> set[str]:
    rules: set[str] = set()

    limits = config.get("limits", {})
    sources = document.get("sources", [])
    text_buffers = document.get("text_buffers", [])
    resources = document.get("resources", {})
    font_records = manifest.get("fonts", [])
    image_records = manifest.get("images", [])
    materialized_states = [
        layout_pass.get("state", {})
        for layout_pass in trace.get("passes", []) if isinstance(layout_pass, dict)
    ]
    epoch_states = [trace.get("initial_state", {}), *materialized_states]

    def exceeds(limit_name: str, observed: int) -> bool:
        limit = limits.get(limit_name)
        return type(limit) is int and observed > limit

    if exceeds("max_ast_nesting_depth", canonical_ast_nesting_depth(document)):
        # This rule is a recursion-safety precheck. Nothing below may traverse
        # an out-of-profile AST before the stable limit result is returned.
        return {"CROSS_LIMIT_AST_NESTING_DEPTH"}

    limit_rules: set[str] = set()
    source_lengths = [
        source.get("utf8_byte_length", 0) for source in sources if isinstance(source, dict)
    ]
    if any(type(length) is int and exceeds("max_source_bytes", length) for length in source_lengths):
        limit_rules.add("CROSS_LIMIT_SOURCE_BYTES")
    if exceeds("max_input_bytes", sum(length for length in source_lengths if type(length) is int)):
        limit_rules.add("CROSS_LIMIT_INPUT_BYTES")
    if exceeds("max_include_files", max(0, len(sources) - 1)):
        limit_rules.add("CROSS_LIMIT_INCLUDE_FILES")
    if exceeds("max_ast_nodes", canonical_ast_node_count(document)):
        limit_rules.add("CROSS_LIMIT_AST_NODES")
    if exceeds("max_style_rules", len(document.get("style_sheet", {}).get("rules", []))):
        limit_rules.add("CROSS_LIMIT_STYLE_RULES")
    text_lengths = [
        len(buffer.get("utf8", "").encode("utf-8"))
        for buffer in text_buffers
        if isinstance(buffer, dict) and isinstance(buffer.get("utf8"), str)
    ]
    overlay_buffer_lengths: list[list[int]] = []
    for state in epoch_states:
        lengths_by_key: dict[tuple[Any, ...], int] = {}
        for record in canonical_generated_records_from_state(state):
            if not isinstance(record, dict):
                continue
            key = generated_key_tuple(record.get("key"))
            utf8 = record.get("utf8")
            if key is not None and isinstance(utf8, str):
                lengths_by_key[key] = lengths_by_key.get(key, 0) + len(utf8.encode("utf-8"))
        overlay_buffer_lengths.append(list(lengths_by_key.values()))
    if any(
        exceeds("max_text_buffer_bytes", length)
        for length in [*text_lengths, *(length for overlay in overlay_buffer_lengths for length in overlay)]
    ):
        limit_rules.add("CROSS_LIMIT_TEXT_BUFFER_BYTES")
    if any(
        exceeds("max_text_bytes", sum(text_lengths) + sum(overlay))
        for overlay in overlay_buffer_lengths or [[]]
    ):
        limit_rules.add("CROSS_LIMIT_TEXT_BYTES")
    if exceeds("max_fonts", len(font_records)):
        limit_rules.add("CROSS_LIMIT_FONT_COUNT")
    if any(exceeds("max_font_bytes", record.get("bytes", 0)) for record in font_records if isinstance(record, dict)):
        limit_rules.add("CROSS_LIMIT_FONT_BYTES")
    if exceeds("max_images", len(image_records)):
        limit_rules.add("CROSS_LIMIT_IMAGE_COUNT")
    if any(exceeds("max_image_bytes", record.get("bytes", 0)) for record in image_records if isinstance(record, dict)):
        limit_rules.add("CROSS_LIMIT_IMAGE_BYTES")
    if any(
        exceeds("max_image_pixels", record.get("pixel_width", 0) * record.get("pixel_height", 0))
        for record in image_records
        if isinstance(record, dict)
        and type(record.get("pixel_width")) is int
        and type(record.get("pixel_height")) is int
    ):
        limit_rules.add("CROSS_LIMIT_IMAGE_PIXELS")
    if any(
        exceeds("max_decoded_image_bytes", record.get("decoded_bytes", 0))
        for record in image_records if isinstance(record, dict)
    ):
        limit_rules.add("CROSS_LIMIT_DECODED_IMAGE_BYTES")
    resource_bytes = sum(
        record.get("bytes", 0)
        for record in [*font_records, *image_records]
        if isinstance(record, dict) and type(record.get("bytes")) is int
    )
    if exceeds("max_resource_bytes", resource_bytes):
        limit_rules.add("CROSS_LIMIT_RESOURCE_BYTES")
    if any(exceeds("max_pages", len(state.get("pages", []))) for state in materialized_states if isinstance(state, dict)):
        limit_rules.add("CROSS_LIMIT_PAGES")
    if any(
        exceeds(
            "max_fragments",
            sum(
                len(page.get("fragments", []))
                for page in state.get("pages", []) if isinstance(page, dict)
            ),
        )
        for state in materialized_states if isinstance(state, dict)
    ):
        limit_rules.add("CROSS_LIMIT_FRAGMENTS")
    output = manifest.get("output")
    if isinstance(output, dict):
        if exceeds("max_pdf_objects", output.get("pdf_object_count", 0)):
            limit_rules.add("CROSS_LIMIT_PDF_OBJECTS")
        if exceeds("max_output_bytes", output.get("bytes", 0)):
            limit_rules.add("CROSS_LIMIT_OUTPUT_BYTES")
    if limit_rules:
        return limit_rules

    if manifest.get("config_sha256") != hashlib.sha256(jcs_bytes(config)).hexdigest():
        rules.add("CROSS_CONFIG_HASH")

    try:
        cargo_workspace = tomllib.loads(
            (REPOSITORY_ROOT / "workspace" / "Cargo.toml").read_text(encoding="utf-8")
        )
        expected_engine = {
            "name": "typaxis",
            "version": cargo_workspace["workspace"]["package"]["version"],
        }
    except (OSError, KeyError, tomllib.TOMLDecodeError):
        expected_engine = None
    engine = manifest.get("engine", {})
    if expected_engine is None or any(engine.get(key) != value for key, value in expected_engine.items()):
        rules.add("CROSS_ENGINE_IDENTITY")

    if manifest.get("stream_compression") != config.get("pdf_stream_compression"):
        rules.add("CROSS_COMPRESSION")

    config_versions = config.get("data_versions", {})
    manifest_versions = manifest.get("data_versions", {})
    if any(
        manifest_versions.get(name) != config_versions.get(name)
        for name in ("unicode", "japanese_line_break")
    ):
        rules.add("CROSS_DATA_VERSIONS")

    if trace.get("max_layout_passes") != config.get("limits", {}).get(
        "max_layout_passes"
    ):
        rules.add("CROSS_MAX_LAYOUT_PASSES")

    trace_result = trace.get("result", {})
    expected_layout = {
        name: trace_result.get(name)
        for name in (
            "status",
            "pass_count",
            "selected_state",
            "final_fingerprint",
            "fallback_policy",
            "flow_registry_sha256",
            "profile_receipt_sha256",
        )
    }
    if manifest.get("layout") != expected_layout:
        rules.add("CROSS_LAYOUT_PROJECTION")

    if manifest.get("status") == "built":
        selected_state = trace_result.get("selected_state")
        passes = trace.get("passes", [])
        output = manifest.get("output")
        if (
            type(selected_state) is int
            and 1 <= selected_state <= len(passes)
            and isinstance(output, dict)
            and output.get("page_count")
            != len(passes[selected_state - 1].get("state", {}).get("pages", []))
        ):
            rules.add("CROSS_SELECTED_PAGE_COUNT")

        if isinstance(output, dict) and output.get("sink") == "file":
            # The sample suite supplies this HostPath externally; manifests never
            # serialize or reconstruct a host output path.
            output_path = artifact_directory / "output.pdf"
            try:
                contents = output_path.read_bytes()
            except OSError:
                rules.add("CROSS_OUTPUT_FACT")
            else:
                try:
                    page_count, object_count = parse_classic_pdf_facts(contents)
                except ValidationFailure:
                    page_count, object_count = -1, -1
                if (
                    output.get("bytes") != len(contents)
                    or output.get("sha256") != hashlib.sha256(contents).hexdigest()
                    or output.get("pdf_object_count") != object_count
                    or output.get("page_count") != page_count
                ):
                    rules.add("CROSS_OUTPUT_FACT")

    document_projection = {
        "algorithm": "typaxis.document-state.sha256/1",
        **{
            name: document.get(name)
            for name in (
                "contract", "coordinate_unit", "sources", "text_buffers", "resources", "document"
            )
        },
    }
    style_projection = {
        "algorithm": "typaxis.style-state.sha256/1",
        **{name: document.get(name) for name in ("page_masters", "style_sheet")},
    }
    declared_fonts_for_epoch = {
        item.get("font_face_id"): item
        for item in document.get("resources", {}).get("font_faces", [])
        if isinstance(item, dict)
    }
    admitted_projection = {
        "algorithm": "typaxis.admitted-resources.jcs-sha256/1",
        "fonts": [
            {
                "font_face_id": record.get("font_face_id"),
                "family": declared_fonts_for_epoch.get(record.get("font_face_id"), {}).get("family"),
                "face_index": record.get("face_index"),
                "sha256": record.get("sha256"),
                "units_per_em": record.get("units_per_em"),
                "glyph_count": record.get("glyph_count"),
            }
            for record in font_records if isinstance(record, dict)
        ],
        "images": [
            {
                name: record.get(name)
                for name in (
                    "image_id", "sha256", "pixel_width", "pixel_height", "decoded_bytes"
                )
            }
            for record in image_records if isinstance(record, dict)
        ],
    }
    expected_epoch = {
        "document_sha256": hashlib.sha256(jcs_bytes(document_projection)).hexdigest(),
        "style_page_master_sha256": hashlib.sha256(jcs_bytes(style_projection)).hexdigest(),
        "admitted_resources_sha256": hashlib.sha256(jcs_bytes(admitted_projection)).hexdigest(),
    }
    if any(
        not isinstance(state, dict)
        or any(state.get("layout_epoch", {}).get(key) != digest for key, digest in expected_epoch.items())
        for state in epoch_states
    ):
        rules.add("CROSS_LAYOUT_EPOCH")

    def file_fact(uri: Any) -> tuple[int, str] | None:
        if not isinstance(uri, str):
            return None
        try:
            contents = (artifact_directory / uri).read_bytes()
        except OSError:
            return None
        return len(contents), hashlib.sha256(contents).hexdigest()

    source_facts = []
    source_catalog_valid = True
    for source in sources:
        if not isinstance(source, dict):
            continue
        fact = file_fact(source.get("uri"))
        expected = (source.get("utf8_byte_length"), source.get("sha256"))
        if fact is None or fact != expected:
            source_catalog_valid = False
        source_facts.append({"uri": source.get("uri"), "bytes": source.get("utf8_byte_length"), "sha256": source.get("sha256")})
    input_records = manifest.get("inputs", [])
    source_facts.sort(key=lambda record: utf8_sort_key(record["uri"]))
    if manifest.get("status") == "built":
        source_catalog_valid &= input_records == source_facts
    else:
        source_catalog_valid &= all(record in source_facts for record in input_records)
    if not source_catalog_valid:
        rules.add("CROSS_SOURCE_CATALOG")

    font_declarations = {
        declaration.get("font_face_id"): declaration
        for declaration in resources.get("font_faces", [])
        if isinstance(declaration, dict)
    } if isinstance(resources, dict) else {}
    image_declarations = {
        declaration.get("image_id"): declaration
        for declaration in resources.get("images", [])
        if isinstance(declaration, dict)
    } if isinstance(resources, dict) else {}
    display_instances = display.get("font_instances", [])
    used_font_instance_ids: set[int] = set()
    used_image_ids: set[int] = set()
    for page in display.get("pages", []):
        if not isinstance(page, dict):
            continue
        for command in page.get("commands", []):
            if not isinstance(command, dict):
                continue
            if command.get("op") == "draw_glyph_run" and type(command.get("font_instance_id")) is int:
                used_font_instance_ids.add(command["font_instance_id"])
            if command.get("op") == "draw_image" and type(command.get("image_id")) is int:
                used_image_ids.add(command["image_id"])
    used_face_ids = {
        item.get("font_face_id")
        for item in display_instances
        if isinstance(item, dict) and item.get("font_instance_id") in used_font_instance_ids
    }
    manifest_fonts_by_id = {
        record.get("font_face_id"): record for record in font_records if isinstance(record, dict)
    }
    declared_font_ids = set(font_declarations)
    font_closure_valid = (
        set(manifest_fonts_by_id) == declared_font_ids
        if manifest.get("status") == "built"
        else set(manifest_fonts_by_id) <= declared_font_ids
    ) and used_face_ids <= set(manifest_fonts_by_id)
    for face_id, manifest_record in manifest_fonts_by_id.items():
        manifest_record = manifest_fonts_by_id.get(face_id)
        declaration = font_declarations.get(face_id)
        if not isinstance(manifest_record, dict) or not isinstance(declaration, dict):
            font_closure_valid = False
            continue
        fact = file_fact(manifest_record.get("uri"))
        expected_hash = declaration.get("expected_sha256")
        if (
            fact != (manifest_record.get("bytes"), manifest_record.get("sha256"))
            or manifest_record.get("uri") != declaration.get("uri")
            or manifest_record.get("face_index") != declaration.get("face_index")
            or (expected_hash is not None and manifest_record.get("sha256") != expected_hash)
        ):
            font_closure_valid = False
    canonical_used_faces = sorted(
        used_face_ids,
        key=lambda face_id: (
            face_id,
            bytes.fromhex(manifest_fonts_by_id[face_id]["sha256"]),
        ),
    ) if used_face_ids <= set(manifest_fonts_by_id) else []
    display_face_order = [
        item.get("font_face_id") for item in display_instances if isinstance(item, dict)
    ]
    if display_face_order != canonical_used_faces:
        font_closure_valid = False
    if not font_closure_valid:
        rules.add("CROSS_FONT_USAGE")

    manifest_images_by_id = {
        record.get("image_id"): record for record in image_records if isinstance(record, dict)
    }
    declared_image_ids = set(image_declarations)
    image_closure_valid = (
        set(manifest_images_by_id) == declared_image_ids
        if manifest.get("status") == "built"
        else set(manifest_images_by_id) <= declared_image_ids
    ) and used_image_ids <= set(manifest_images_by_id)
    for image_id in manifest_images_by_id:
        manifest_record = manifest_images_by_id.get(image_id)
        declaration = image_declarations.get(image_id)
        if not isinstance(manifest_record, dict) or not isinstance(declaration, dict):
            image_closure_valid = False
            continue
        fact = file_fact(manifest_record.get("uri"))
        expected_hash = declaration.get("expected_sha256")
        if (
            fact != (manifest_record.get("bytes"), manifest_record.get("sha256"))
            or manifest_record.get("uri") != declaration.get("uri")
            or (expected_hash is not None and manifest_record.get("sha256") != expected_hash)
        ):
            image_closure_valid = False
    if not image_closure_valid:
        rules.add("CROSS_IMAGE_USAGE")

    nodes_by_id = {
        node.get("node_id"): node
        for node in typed_document_preorder(document.get("document", {}))
    }
    known_nodes = set(nodes_by_id)
    node_paths = typed_document_paths(document.get("document", {}))
    node_kinds = typed_document_node_kinds(document.get("document", {}))
    known_footnotes = {
        footnote.get("footnote_id")
        for footnote in document.get("document", {}).get("footnotes", [])
        if isinstance(footnote, dict)
    }
    known_masters = {
        master.get("master_id"): master
        for master in document.get("page_masters", {}).get("masters", [])
        if isinstance(master, dict)
    }
    document_anchor_owners = {
        node.get("anchor_id"): node.get("node_id")
        for node in typed_document_preorder(document.get("document", {}))
        if node.get("kind") == "anchor"
        or (node.get("kind") == "heading" and node.get("anchor_id") is not None)
    }
    trace_node_valid = True
    trace_flow_owner_valid = True
    generated_key_valid = True
    trace_frame_valid = True
    trace_footnotes_valid = True
    trace_master_valid = True
    reference_target_valid = True
    trace_anchor_valid = True
    generated_sites = expected_generated_sites(document)
    for epoch_state in epoch_states:
        observed_site_keys: set[tuple[Any, ...]] = set()
        for generated in canonical_generated_records_from_state(epoch_state):
            key = generated.get("key", {}) if isinstance(generated, dict) else {}
            if not isinstance(key, dict):
                continue
            key_tuple = generated_key_tuple(key)
            if key_tuple is not None:
                observed_site_keys.add(key_tuple)
            if key.get("owner") not in nodes_by_id:
                trace_node_valid = False
            if key_tuple not in generated_sites:
                generated_key_valid = False
        if observed_site_keys != set(generated_sites):
            generated_key_valid = False
        state_positions = epoch_state.get("flow_positions", []) if isinstance(epoch_state, dict) else []
        for position in state_positions:
            if isinstance(position, dict) and (
                position.get("owner") not in known_nodes
                or tuple(position.get("block_child_path", []))
                != node_paths.get(position.get("owner"))
            ):
                trace_node_valid = False
        owner_boundaries: dict[Any, list[Any]] = {}
        for position in state_positions[1:-1] if isinstance(state_positions, list) else []:
            if not isinstance(position, dict):
                trace_flow_owner_valid = False
                continue
            owner = position.get("owner")
            kind = node_kinds.get(owner)
            if kind not in {
                "paragraph", "heading", "list_item", "table_row",
                "figure", "page_break",
            }:
                trace_flow_owner_valid = False
                continue
            owner_boundaries.setdefault(owner, []).append(
                position.get("owner_local_boundary")
            )
        for owner, boundaries in owner_boundaries.items():
            kind = node_kinds.get(owner)
            if kind in {"list_item", "table_row", "figure", "page_break"}:
                if boundaries != [0]:
                    trace_flow_owner_valid = False
            elif boundaries != list(range(len(boundaries))):
                trace_flow_owner_valid = False
    for state in materialized_states:
        if not isinstance(state, dict):
            continue
        overlay_records = canonical_generated_records_from_state(state)
        overlay_identity = {
            (
                generated_key_tuple(record.get("key")),
                record.get("start_byte"),
                record.get("end_byte"),
                record.get("utf8"),
            )
            for record in overlay_records if isinstance(record, dict)
        }
        state_footnotes: set[Any] = set()
        observed_anchors: dict[Any, Any] = {}
        placed_anchors = state.get("placed_anchors", [])
        if not isinstance(placed_anchors, list):
            trace_anchor_valid = False
            placed_anchors = []
        for anchor in placed_anchors:
            if (
                not isinstance(anchor, dict)
                or anchor.get("anchor_id") in observed_anchors
            ):
                trace_anchor_valid = False
                continue
            observed_anchors[anchor.get("anchor_id")] = anchor.get("owner")
        if observed_anchors != document_anchor_owners:
            trace_anchor_valid = False
        for page in state.get("pages", []):
            if not isinstance(page, dict):
                continue
            if page.get("master_id") not in known_masters:
                trace_master_valid = False
                master = None
            else:
                master = known_masters[page.get("master_id")]
            for frame in page.get("frames", []):
                if not isinstance(frame, dict) or not isinstance(master, dict):
                    continue
                region = master.get(frame.get("kind"))
                if not isinstance(region, dict) or not rect_contains(region, frame.get("bounds")):
                    trace_frame_valid = False
            state_footnotes.update(page.get("footnote_ids", []))
            for fragment in page.get("fragments", []):
                if isinstance(fragment, dict) and fragment.get("owner") not in known_nodes:
                    trace_node_valid = False
                if isinstance(fragment, dict):
                    for position_name in ("start", "end"):
                        position = fragment.get(position_name, {})
                        if isinstance(position, dict) and position.get("owner") not in known_nodes:
                            trace_node_valid = False
            for decision in page.get("float_decisions", []):
                if isinstance(decision, dict) and decision.get("owner") not in known_nodes:
                    trace_node_valid = False
            for decision in page.get("column_decisions", []):
                if isinstance(decision, dict) and decision.get("container") not in known_nodes:
                    trace_node_valid = False
            for reference in page.get("resolved_references", []):
                if not isinstance(reference, dict):
                    continue
                key = reference.get("buffer_key", {})
                key_tuple = generated_key_tuple(key)
                site = generated_sites.get(key_tuple) if key_tuple is not None else None
                if isinstance(key, dict) and key.get("owner") not in known_nodes:
                    trace_node_valid = False
                elif (
                    not isinstance(site, dict)
                    or site.get("generation_kind") != "page_reference"
                ):
                    generated_key_valid = False
                elif site.get("target") != reference.get("anchor_id"):
                    reference_target_valid = False
                if reference.get("anchor_id") not in {
                    node.get("anchor_id")
                    for node in typed_document_preorder(document.get("document", {}))
                    if node.get("kind") == "anchor"
                    or (node.get("kind") == "heading" and node.get("anchor_id") is not None)
                }:
                    trace_node_valid = False
                if (
                    generated_key_tuple(key), reference.get("start_byte"),
                    reference.get("end_byte"), reference.get("utf8")
                ) not in overlay_identity:
                    generated_key_valid = False
        referenced_footnotes = {
            node.get("footnote_id")
            for node in typed_document_preorder(document.get("document", {}))
            if node.get("kind") == "footnote_reference"
        }
        if state_footnotes != referenced_footnotes:
            trace_footnotes_valid = False
    if not trace_node_valid:
        rules.add("CROSS_TRACE_NODE")
    if not trace_flow_owner_valid:
        rules.add("CROSS_FLOW_OWNER")
    if not trace_master_valid:
        rules.add("CROSS_TRACE_MASTER")
    if not trace_frame_valid:
        rules.add("CROSS_TRACE_FRAME")
    if not generated_key_valid:
        rules.add("CROSS_GENERATED_KEY")
    if not trace_footnotes_valid:
        rules.add("CROSS_TRACE_FOOTNOTE")
    if not reference_target_valid:
        rules.add("CROSS_REFERENCE_TARGET")
    if not trace_anchor_valid:
        rules.add("CROSS_TRACE_ANCHOR")

    document_anchors = set(document_anchor_owners)
    display_anchors = {
        destination.get("anchor_id")
        for destination in display.get("destinations", []) if isinstance(destination, dict)
    }
    if display_anchors != document_anchors:
        rules.add("CROSS_DISPLAY_ANCHOR")

    selected_state = trace_result.get("selected_state")
    passes = trace.get("passes", [])
    if type(selected_state) is int and 1 <= selected_state <= len(passes):
        selected_pass = passes[selected_state - 1]
        selected_record = selected_pass.get("state", {}) if isinstance(selected_pass, dict) else {}
        source_layout = display.get("source_layout", {})
        if (
            not isinstance(source_layout, dict)
            or source_layout.get("state_fingerprint") != selected_pass.get("output_fingerprint")
            or source_layout.get("layout_epoch") != selected_record.get("layout_epoch")
        ):
            rules.add("CROSS_DISPLAY_LAYOUT")
        parsed_text = {
            item.get("text_id"): item.get("utf8")
            for item in document.get("text_buffers", []) if isinstance(item, dict)
        }
        generated_by_key: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
        for item in canonical_generated_records_from_state(
            selected_record
        ):
            if not isinstance(item, dict):
                continue
            key = generated_key_tuple(item.get("key"))
            if key is not None:
                generated_by_key.setdefault(key, []).append(item)
        generated_text: dict[tuple[Any, ...], str] = {}
        for key, records in generated_by_key.items():
            records.sort(key=lambda item: (item.get("start_byte"), item.get("end_byte")))
            expected_start = 0
            valid = True
            pieces: list[str] = []
            for item in records:
                if item.get("start_byte") != expected_start or not isinstance(item.get("utf8"), str):
                    valid = False
                    break
                expected_start = item.get("end_byte")
                pieces.append(item["utf8"])
            if valid:
                generated_text[key] = "".join(pieces)
        display_text_valid = True
        for item in display.get("text_buffers", []):
            if not isinstance(item, dict):
                continue
            origin = item.get("origin", {})
            expected_text = None
            if isinstance(origin, dict) and origin.get("kind") == "parsed":
                expected_text = parsed_text.get(origin.get("text_buffer_id"))
            elif isinstance(origin, dict) and origin.get("kind") == "generated":
                key = generated_key_tuple(origin.get("key"))
                expected_text = generated_text.get(key) if key is not None else None
            if expected_text is None or item.get("utf8") != expected_text:
                display_text_valid = False
        if not display_text_valid and font_closure_valid:
            rules.add("CROSS_DISPLAY_TEXT")

        selected_pages = selected_record.get("pages", [])
        display_pages = display.get("pages", [])
        page_closure_valid = len(selected_pages) == len(display_pages)
        for trace_page, display_page in zip(selected_pages, display_pages):
            master = known_masters.get(trace_page.get("master_id")) if isinstance(trace_page, dict) else None
            if (
                not isinstance(master, dict)
                or not isinstance(display_page, dict)
                or trace_page.get("page_index") != display_page.get("page_index")
                or display_page.get("width") != master.get("width")
                or display_page.get("height") != master.get("height")
            ):
                page_closure_valid = False
        if not page_closure_valid and trace_master_valid:
            rules.add("CROSS_DISPLAY_PAGE")

        frame_bounds_by_key: dict[tuple[Any, Any, Any], dict[str, Any]] = {}
        for page in selected_pages if isinstance(selected_pages, list) else []:
            if not isinstance(page, dict):
                continue
            for frame in page.get("frames", []):
                if isinstance(frame, dict):
                    frame_bounds_by_key[(
                        page.get("page_index"), frame.get("kind"), frame.get("column_index")
                    )] = frame.get("bounds", {})
        expected_destinations: list[dict[str, Any]] = []
        selected_anchors = selected_record.get("placed_anchors", [])
        destination_projection_valid = isinstance(selected_anchors, list)
        for anchor in selected_anchors if isinstance(selected_anchors, list) else []:
            if not isinstance(anchor, dict):
                destination_projection_valid = False
                continue
            frame_bounds = frame_bounds_by_key.get((
                anchor.get("page_index"), anchor.get("frame_kind"), anchor.get("column_index")
            ))
            local = anchor.get("position_in_frame")
            if (
                not isinstance(frame_bounds, dict)
                or not isinstance(local, dict)
                or not all(type(frame_bounds.get(axis)) is int for axis in ("x", "y"))
                or not all(type(local.get(axis)) is int for axis in ("x", "y"))
            ):
                destination_projection_valid = False
                continue
            expected_destinations.append({
                "anchor_id": anchor.get("anchor_id"),
                "page_index": anchor.get("page_index"),
                "view": {
                    "kind": "xyz",
                    "point": {
                        "x": frame_bounds["x"] + local["x"],
                        "y": frame_bounds["y"] + local["y"],
                    },
                },
            })
        if (
            not destination_projection_valid
            or display.get("destinations") != expected_destinations
        ):
            rules.add("CROSS_DISPLAY_ANCHOR")

    if (
        config.get("strict") is True
        and trace_result.get("status") in {"cycle_fallback", "max_pass_fallback"}
        and manifest.get("status") == "built"
    ):
        rules.add("CROSS_STRICT_FALLBACK_BUILD")
    return rules


def materialize_cross_fixture(
    raw_fixture: Any,
    config: dict[str, Any],
    document: dict[str, Any],
    display: dict[str, Any],
    trace: dict[str, Any],
    manifest: dict[str, Any],
    label: str,
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
    if (
        not isinstance(raw_fixture, dict)
        or set(raw_fixture) != {"$fixture", "mutations"}
        or raw_fixture.get("$fixture") != "cross_artifacts"
    ):
        raise ValidationFailure(
            f"{label}: cross fixture must contain $fixture=cross_artifacts and mutations"
        )
    documents = {
        "config": copy.deepcopy(config),
        "document": copy.deepcopy(document),
        "display": copy.deepcopy(display),
        "trace": copy.deepcopy(trace),
        "manifest": copy.deepcopy(manifest),
    }
    mutations = raw_fixture["mutations"]
    if not isinstance(mutations, list) or not mutations:
        raise ValidationFailure(f"{label}: mutations must be a non-empty array")
    for mutation_index, mutation in enumerate(mutations):
        mutation_label = f"{label}: mutation {mutation_index}"
        if not isinstance(mutation, dict) or set(mutation) != {
            "artifact",
            "path",
            "value",
        }:
            raise ValidationFailure(
                f"{mutation_label}: mutation must contain artifact, path, and value"
            )
        artifact = mutation["artifact"]
        if artifact not in CROSS_ARTIFACT_NAMES:
            raise ValidationFailure(f"{mutation_label}: unknown artifact {artifact!r}")
        documents[artifact] = apply_fixture_mutations(
            documents[artifact],
            [{"path": mutation["path"], "value": mutation["value"]}],
            mutation_label,
        )
    return (
        documents["config"], documents["document"], documents["display"],
        documents["trace"], documents["manifest"],
    )


def conformance_rule_ids(
    schema_name: str, instance: Any, effective_config: dict[str, Any] | None = None
) -> set[str]:
    if not isinstance(instance, dict):
        return set()
    if contains_non_scalar_string(instance):
        return {"JSON_UNICODE_SCALAR"}
    if schema_name == "machine-capabilities.schema.json":
        return capabilities_rule_ids(instance)
    if schema_name == "package-config.schema.json":
        return config_rule_ids(instance)
    if schema_name == "diagnostics.schema.json":
        return {
            "DIAGNOSTIC_CODE"
            for diagnostic in instance.get("diagnostics", [])
            if not isinstance(diagnostic, dict)
            or not isinstance(diagnostic.get("code"), str)
            or DIAGNOSTIC_CODE.fullmatch(diagnostic["code"]) is None
        }
    if schema_name == "display-list.schema.json":
        allowed_schemes = (
            set(effective_config.get("allowed_uri_schemes", []))
            if effective_config is not None
            else None
        )
        max_uri_bytes = (
            effective_config.get("limits", {}).get("max_uri_bytes")
            if effective_config is not None
            else None
        )
        return display_rule_ids(instance, allowed_schemes, max_uri_bytes)
    if schema_name == "document-package.schema.json":
        allowed_schemes = (
            set(effective_config.get("allowed_uri_schemes", []))
            if effective_config is not None
            else None
        )
        document_max_uri_bytes = (
            effective_config.get("limits", {}).get("max_uri_bytes")
            if effective_config is not None
            else None
        )
        document_max_ast_nesting_depth = (
            effective_config.get("limits", {}).get("max_ast_nesting_depth")
            if effective_config is not None
            else None
        )
        return document_rule_ids(
            instance,
            allowed_schemes,
            document_max_uri_bytes,
            document_max_ast_nesting_depth,
        )
    if schema_name == "build-manifest.schema.json":
        return manifest_rule_ids(instance)
    if schema_name == "layout-trace.schema.json":
        return trace_rule_ids(instance)
    return set()


def load_schema_registry(
    directory: Path, version: str
) -> tuple[dict[str, Any], dict[str, Draft202012Validator], int]:
    schemas = {
        path.name: load_json(path)
        for path in sorted(directory.glob("*.schema.json"))
    }
    if not schemas:
        raise ValidationFailure(f"{directory}: no schemas were found")
    prefix = f"https://schemas.typaxis.invalid/{version}/"
    for filename, schema in schemas.items():
        schema_id = schema.get("$id")
        if not isinstance(schema_id, str) or not schema_id.startswith(prefix):
            raise ValidationFailure(
                f"{directory / filename}: $id is outside the {version} registry"
            )
        Draft202012Validator.check_schema(schema)
    reference_count = validate_references(schemas)
    registry = Registry().with_resources(
        (schema["$id"], Resource.from_contents(schema))
        for schema in schemas.values()
    )
    validators = {
        filename: Draft202012Validator(schema, registry=registry)
        for filename, schema in schemas.items()
    }
    return schemas, validators, reference_count


def validate_staging_multi_flow_bundle(
    trace: dict[str, Any], manifest: dict[str, Any]
) -> None:
    positions = trace.get("flow_positions")
    if not isinstance(positions, list) or not positions:
        raise ValidationFailure("1.2 staging trace has no flow positions")
    grouped: dict[int, list[dict[str, Any]]] = {}
    previous_flow = -1
    epoch = positions[0].get("epoch") if isinstance(positions[0], dict) else None
    for position in positions:
        if not isinstance(position, dict):
            raise ValidationFailure("1.2 staging flow position is not an object")
        flow_id = position.get("flow_id")
        if type(flow_id) is not int or flow_id < previous_flow:
            raise ValidationFailure("1.2 staging flow positions are not in flow order")
        previous_flow = flow_id
        if position.get("epoch") != epoch:
            raise ValidationFailure("1.2 staging flow positions mix layout epochs")
        grouped.setdefault(flow_id, []).append(position)
    if list(grouped) != list(range(len(grouped))):
        raise ValidationFailure("1.2 staging FlowIds are not dense")

    derived_flows: list[dict[str, Any]] = []
    for flow_id, flow_positions in grouped.items():
        if [position.get("flow_local_ordinal") for position in flow_positions] != list(
            range(len(flow_positions))
        ):
            raise ValidationFailure("1.2 staging flow ordinals are not dense")
        terminals = [position for position in flow_positions if position.get("terminal") is True]
        if len(terminals) != 1 or terminals[0] is not flow_positions[-1]:
            raise ValidationFailure("1.2 staging flow lacks one final terminal")
        owner_node_id = flow_positions[0].get("owner_node_id")
        parent_flow_id = flow_positions[0].get("parent_flow_id")
        if any(
            position.get("owner_node_id") != owner_node_id
            or position.get("parent_flow_id") != parent_flow_id
            for position in flow_positions
        ):
            raise ValidationFailure("1.2 staging flow relation changes within a flow")
        if flow_id == 0:
            if owner_node_id != 0 or parent_flow_id is not None:
                raise ValidationFailure("1.2 staging body flow relation is invalid")
        elif type(parent_flow_id) is not int or not 0 <= parent_flow_id < flow_id:
            raise ValidationFailure("1.2 staging child flow parent is not earlier")
        derived_flows.append(
            {
                "flow_id": flow_id,
                "owner_node_id": owner_node_id,
                "parent_flow_id": parent_flow_id,
                "terminal": terminals[0]["flow_local_ordinal"],
            }
        )

    for position in positions:
        child_flow_id = position.get("child_flow_id")
        if child_flow_id is None:
            continue
        flow_id = position["flow_id"]
        if (
            type(child_flow_id) is not int
            or child_flow_id <= flow_id
            or child_flow_id >= len(derived_flows)
            or derived_flows[child_flow_id]["parent_flow_id"] != flow_id
            or derived_flows[child_flow_id]["owner_node_id"]
            != position.get("content_owner_node_id")
        ):
            raise ValidationFailure("1.2 staging child flow edge is invalid")

    if manifest.get("flows") != derived_flows:
        raise ValidationFailure("1.2 staging manifest does not cover exact flow terminals")
    if manifest.get("flow_registry_sha256") != trace.get("flow_registry_sha256"):
        raise ValidationFailure("1.2 staging registry hashes differ")
    if manifest.get("selected_state_sha256") != trace.get("selected_state_sha256"):
        raise ValidationFailure("1.2 staging selected-state hashes differ")


def validate_staging_machine_list_bundle(
    package: dict[str, Any], selected: dict[str, Any], expectation: dict[str, Any]
) -> None:
    """Close the MI2-04 fixture across package semantics and selected facts."""

    if expectation != {
        "contract": "typaxis.contract/1.2",
        "profile": "typaxis.machine-pdf/basic-document-1",
        "policy_version": "typaxis.basic-list-policy/1",
        "scenarios": {
            "deterministic_double_build": True,
            "empty_painted_item": "L5100",
            "exact_marker_buffer_bytes": 3,
            "marker_buffer_max_plus_one": 2,
            "marker_overflow": "L5100",
            "nested": True,
            "page_split": True,
            "single": True,
            "tamper": ["missing_item", "extra_item", "wrong_item"],
        },
    }:
        raise ValidationFailure("MI2-04 scenario expectation drifted")
    if selected.get("policy_version") != expectation["policy_version"]:
        raise ValidationFailure("MI2-04 list policy differs between expectation and facts")

    list_style: dict[str, Any] | None = None
    for rule in package.get("style_sheet", {}).get("rules", []):
        if rule.get("selector") == "list":
            if list_style is not None:
                raise ValidationFailure("MI2-04 fixture has an ambiguous list style")
            list_style = {
                declaration.get("name"): declaration.get("value", {}).get("value")
                for declaration in rule.get("declarations", [])
            }
    if list_style is None:
        raise ValidationFailure("MI2-04 fixture has no exact list style")
    expected_font_size = list_style.get("font_size")
    expected_start_indent = list_style.get("start_indent")
    expected_end_indent = list_style.get("end_indent")
    if any(
        type(value) is not int or value < 0
        for value in (expected_font_size, expected_start_indent, expected_end_indent)
    ) or expected_font_size == 0:
        raise ValidationFailure("MI2-04 fixture list geometry style is incomplete")

    lists: dict[int, dict[str, Any]] = {}
    expected_items: dict[int, dict[str, Any]] = {}
    pending: list[tuple[dict[str, Any], int | None]] = [
        (block, None)
        for block in reversed(package.get("document", {}).get("blocks", []))
    ]
    while pending:
        block, containing_item = pending.pop()
        kind = block.get("kind")
        if kind == "list":
            list_node_id = block.get("node_id")
            if type(list_node_id) is not int or list_node_id in lists:
                raise ValidationFailure("MI2-04 fixture has a duplicate or invalid list owner")
            ordered = block.get("ordered")
            start = block.get("start")
            if type(ordered) is not bool or (ordered and type(start) is not int):
                raise ValidationFailure("MI2-04 fixture has an invalid closed list kind")
            if not ordered and start is not None:
                raise ValidationFailure("MI2-04 unordered list has a start value")
            items = block.get("items")
            if not isinstance(items, list) or not items:
                raise ValidationFailure("MI2-04 fixture has an empty list")
            lists[list_node_id] = {"containing_item": containing_item}
            for item_index, item in enumerate(items):
                item_node_id = item.get("node_id")
                if type(item_node_id) is not int or item_node_id in expected_items:
                    raise ValidationFailure(
                        "MI2-04 fixture has a duplicate or invalid list-item owner"
                    )
                if ordered:
                    marker_value = start + item_index
                    if marker_value > 4_294_967_295:
                        raise ValidationFailure("MI2-04 fixture ordered marker overflows u32")
                    marker_utf8 = f"{marker_value}."
                else:
                    marker_utf8 = "\u2022"
                expected_items[item_node_id] = {
                    "item_index": item_index,
                    "list_node_id": list_node_id,
                    "marker_utf8": marker_utf8,
                }
            for item in reversed(items):
                item_node_id = item["node_id"]
                pending.extend(
                    (child, item_node_id)
                    for child in reversed(item.get("blocks", []))
                )
        elif kind == "figure":
            pending.extend(
                (child, containing_item)
                for child in reversed(block.get("caption", []))
            )
        elif kind == "table":
            rows = [*block.get("head", []), *block.get("body", [])]
            table_blocks = [
                child
                for row in rows
                for cell in row.get("cells", [])
                for child in cell.get("blocks", [])
            ]
            pending.extend(
                (child, containing_item) for child in reversed(table_blocks)
            )

    observed_items = selected.get("items")
    observed_lists = selected.get("list_flows")
    if not isinstance(observed_items, list) or not isinstance(observed_lists, list):
        raise ValidationFailure("MI2-04 selected facts omit list closure")
    if [item.get("item_node_id") for item in observed_items] != sorted(expected_items):
        raise ValidationFailure("MI2-04 selected items are missing, extra, or noncanonical")
    if [item.get("list_node_id") for item in observed_lists] != sorted(lists):
        raise ValidationFailure("MI2-04 selected list facts are missing, extra, or noncanonical")

    observed_by_item = {item["item_node_id"]: item for item in observed_items}
    observed_by_list = {item["list_node_id"]: item for item in observed_lists}
    if len(observed_by_item) != len(observed_items) or len(observed_by_list) != len(observed_lists):
        raise ValidationFailure("MI2-04 selected facts contain duplicate owners")

    item_flow_ids = [item.get("item_flow_id") for item in observed_items]
    if item_flow_ids != list(range(1, len(observed_items) + 1)):
        raise ValidationFailure("MI2-04 list-item FlowIds are not dense and canonical")

    all_fragments: list[int] = []
    previous_page = -1
    for item_node_id in sorted(expected_items):
        expected = expected_items[item_node_id]
        item = observed_by_item[item_node_id]
        for field in ("item_index", "list_node_id", "marker_utf8"):
            if item.get(field) != expected[field]:
                raise ValidationFailure(
                    f"MI2-04 selected item has wrong derived {field}: {item_node_id}"
                )
        marker_key = item.get("marker_key")
        if marker_key != {
            "generation_kind": "list_marker",
            "owner": item_node_id,
            "owner_local_ordinal": 0,
        }:
            raise ValidationFailure("MI2-04 marker key is not item-bound and canonical")
        list_fact = observed_by_list[expected["list_node_id"]]
        if item.get("list_flow_id") != list_fact.get("list_flow_id"):
            raise ValidationFailure("MI2-04 item/list FlowId closure is broken")
        if item.get("marker_column_width") != list_fact.get("marker_column_width"):
            raise ValidationFailure("MI2-04 item/list marker-column closure is broken")
        if item.get("content_inline_size") != list_fact.get("item_frame_inline_size"):
            raise ValidationFailure("MI2-04 item/list inline-size closure is broken")
        expected_marker_left = (
            list_fact["start_indent"]
            + list_fact["marker_column_width"]
            - item["marker_inline_size"]
        )
        if item.get("marker_physical_left") != expected_marker_left:
            raise ValidationFailure("MI2-04 marker is not end-aligned in the widest column")
        expected_content_left = (
            list_fact["start_indent"]
            + list_fact["marker_column_width"]
            + list_fact["marker_gap"]
        )
        if item.get("content_physical_left") != expected_content_left:
            raise ValidationFailure("MI2-04 marker gap/content placement is not exact")
        fragments = item.get("fragment_ids")
        if (
            not isinstance(fragments, list)
            or not fragments
            or fragments[0] != item.get("marker_fragment_id")
            or item.get("marker_fragment_id") != item.get("first_line_fragment_id")
        ):
            raise ValidationFailure("MI2-04 marker and first painted line are orphaned")
        if fragments != sorted(set(fragments)):
            raise ValidationFailure("MI2-04 item fragments are not strictly ordered")
        all_fragments.extend(fragments)
        page_index = item.get("page_index")
        if type(page_index) is not int or page_index < previous_page:
            raise ValidationFailure("MI2-04 item pages are not canonical")
        previous_page = page_index
        if page_index >= selected.get("page_count", 0):
            raise ValidationFailure("MI2-04 item page is outside the selected page count")

    if all_fragments != list(range(len(all_fragments))):
        raise ValidationFailure("MI2-04 fragment IDs are not globally dense")
    if selected.get("page_count") != previous_page + 1:
        raise ValidationFailure("MI2-04 selected page count does not close page-split facts")

    marker_widths: dict[int, list[int]] = {list_node_id: [] for list_node_id in lists}
    for item in observed_items:
        marker_widths[item["list_node_id"]].append(item["marker_inline_size"])
    for list_node_id in sorted(lists):
        list_fact = observed_by_list[list_node_id]
        if list_fact.get("marker_column_width") != max(marker_widths[list_node_id]):
            raise ValidationFailure("MI2-04 marker column is not the widest marker")
        if (
            list_fact.get("marker_gap") != expected_font_size
            or list_fact.get("start_indent") != expected_start_indent
            or list_fact.get("end_indent") != expected_end_indent
        ):
            raise ValidationFailure("MI2-04 list geometry does not match computed style")
        containing_item = lists[list_node_id]["containing_item"]
        containing_frame_inline_size = (
            package["page_masters"]["masters"][0]["body"]["width"]
            if containing_item is None
            else observed_by_item[containing_item]["content_inline_size"]
        )
        if (
            list_fact["start_indent"]
            + list_fact["marker_column_width"]
            + list_fact["marker_gap"]
            + list_fact["item_frame_inline_size"]
            + list_fact["end_indent"]
            != containing_frame_inline_size
        ):
            raise ValidationFailure("MI2-04 list frame does not close its containing flow")
        expected_list_flow = (
            0 if containing_item is None else observed_by_item[containing_item]["item_flow_id"]
        )
        if list_fact.get("list_flow_id") != expected_list_flow:
            raise ValidationFailure("MI2-04 nested list is not bound to its parent item flow")


def validate_staging_forced_page_break_bundle(
    package: dict[str, Any],
    trace: dict[str, Any],
    selected: dict[str, Any],
    expectation: dict[str, Any],
) -> None:
    """Close the MI2-05 forced-boundary, cursor, and blank-page fixture."""

    if expectation != {
        "contract": "typaxis.contract/1.2",
        "profile": "typaxis.machine-pdf/basic-document-1",
        "policy_version": "typaxis.basic-forced-page-break-policy/1",
        "scenarios": {
            "blank_page_indexes": [0, 2, 4],
            "consecutive": True,
            "cursor_tamper": "I9190",
            "deterministic_double_build": True,
            "leading": True,
            "max_pages": 5,
            "max_plus_one": "limit",
            "middle": True,
            "painted_content_owners": [2, 5],
            "trailing": True,
        },
    }:
        raise ValidationFailure("MI2-05 scenario expectation drifted")
    if selected.get("policy_version") != expectation["policy_version"]:
        raise ValidationFailure("MI2-05 blank-page policy differs between fixture and facts")
    for field in (
        "break_usage_sha256",
        "flow_registry_sha256",
        "forced_page_breaks",
        "page_count",
        "pages",
        "policy_version",
    ):
        if trace.get(field) != selected.get(field):
            raise ValidationFailure(f"MI2-05 trace/manifest {field} closure is broken")

    blocks = package.get("document", {}).get("blocks", [])
    if not isinstance(blocks, list):
        raise ValidationFailure("MI2-05 document blocks are missing")
    break_owners: list[int] = []
    expected_before_ordinals: list[int] = []
    for flow_local_ordinal, block in enumerate(blocks):
        if block.get("kind") == "page_break":
            owner = block.get("node_id")
            if type(owner) is not int:
                raise ValidationFailure("MI2-05 break owner is invalid")
            break_owners.append(owner)
            expected_before_ordinals.append(flow_local_ordinal)

    receipts = selected.get("forced_page_breaks")
    if not isinstance(receipts, list) or not receipts:
        raise ValidationFailure("MI2-05 selected facts omit forced page breaks")
    if [receipt.get("break_node_id") for receipt in receipts] != break_owners:
        raise ValidationFailure("MI2-05 break coverage is missing, extra, or noncanonical")
    if len(set(break_owners)) != len(break_owners):
        raise ValidationFailure("MI2-05 fixture has duplicate break owners")

    for index, (receipt, before_ordinal) in enumerate(
        zip(receipts, expected_before_ordinals, strict=True)
    ):
        before = receipt.get("before_cursor")
        after = receipt.get("after_cursor")
        if receipt.get("document_ordinal") != index:
            raise ValidationFailure("MI2-05 break document ordinals are not dense")
        if receipt.get("produced_page_index") != index + 1:
            raise ValidationFailure("MI2-05 break did not produce exactly one next page")
        if before != {"flow_id": 0, "flow_local_ordinal": before_ordinal}:
            raise ValidationFailure("MI2-05 pre-break cursor is not flow-bound")
        if after != {"flow_id": 0, "flow_local_ordinal": before_ordinal + 1}:
            raise ValidationFailure("MI2-05 break cursor did not advance exactly once")

    pages = selected.get("pages")
    page_count = selected.get("page_count")
    if page_count != len(receipts) + 1 or not isinstance(pages, list):
        raise ValidationFailure("MI2-05 N-break to N+1-page policy is broken")
    if len(pages) != page_count:
        raise ValidationFailure("MI2-05 PDF page count does not close selected pages")
    if [page.get("page_index") for page in pages] != list(range(page_count)):
        raise ValidationFailure("MI2-05 page indexes are not dense")
    for page in pages:
        painted = page.get("painted_content_count")
        if type(painted) is not int or painted < 0:
            raise ValidationFailure("MI2-05 painted-content count is invalid")
        if page.get("is_blank") is not (painted == 0):
            raise ValidationFailure("MI2-05 blank page is inconsistent with painted content")
    blank_pages = [page["page_index"] for page in pages if page["is_blank"]]
    if blank_pages != expectation["scenarios"]["blank_page_indexes"]:
        raise ValidationFailure("MI2-05 leading/consecutive/trailing blank pages drifted")
    if page_count != expectation["scenarios"]["max_pages"]:
        raise ValidationFailure("MI2-05 exact max-pages fixture drifted")


def validate_staging_machine_figure_bundle(
    package: dict[str, Any],
    selected: dict[str, Any],
    expectation: dict[str, Any],
    png_bytes: bytes,
) -> None:
    """Close MI2-06 from declared image identity through PDF XObject facts."""

    if expectation != {
        "contract": "typaxis.contract/1.2",
        "profile": "typaxis.machine-pdf/basic-document-1",
        "policy_version": "typaxis.basic-png-figure-policy/1",
        "scenarios": {
            "bad_hash": "resource_hash_mismatch",
            "caption_block_sizes": [15, 15],
            "caption_keep": True,
            "caption_split": True,
            "deterministic_double_build": True,
            "image_xobject_count": 2,
            "initial_consumed_block_size": 50,
            "invalid_dimensions": "invalid_metadata",
            "non_png": "invalid_metadata",
            "oversize": "terminal_once",
            "pixel_limit": "resource_limit",
            "publication_failure": "no_success",
            "wrong_image_id": "draw_image_closure",
        },
    }:
        raise ValidationFailure("MI2-06 scenario expectation drifted")
    if (
        selected.get("contract") != expectation["contract"]
        or selected.get("profile") != expectation["profile"]
        or selected.get("policy_version") != expectation["policy_version"]
    ):
        raise ValidationFailure("MI2-06 contract/profile/policy closure drifted")

    declarations = package.get("resources", {}).get("images")
    if not isinstance(declarations, list) or len(declarations) != 1:
        raise ValidationFailure("MI2-06 fixture must declare exactly one image")
    declaration = declarations[0]
    if set(declaration) != {"expected_sha256", "image_id", "uri"}:
        raise ValidationFailure("MI2-06 image declaration gained a media assertion")
    if declaration.get("uri", "").lower().endswith(".png"):
        raise ValidationFailure("MI2-06 fixture accidentally permits suffix inference")
    if hashlib.sha256(png_bytes).hexdigest() != declaration.get("expected_sha256"):
        raise ValidationFailure("MI2-06 admitted PNG hash does not match its declaration")
    if (
        len(png_bytes) < 29
        or png_bytes[:8] != b"\x89PNG\r\n\x1a\n"
        or int.from_bytes(png_bytes[8:12], "big") != 13
        or png_bytes[12:16] != b"IHDR"
    ):
        raise ValidationFailure("MI2-06 fixture payload is not a PNG IHDR")
    pixel_width = int.from_bytes(png_bytes[16:20], "big")
    pixel_height = int.from_bytes(png_bytes[20:24], "big")
    bit_depth = png_bytes[24]
    color_type = png_bytes[25]
    if pixel_width == 0 or pixel_height == 0:
        raise ValidationFailure("MI2-06 fixture PNG dimensions are invalid")
    decoded_bytes_per_pixel = {
        (0, 1): 2,
        (0, 2): 2,
        (0, 4): 2,
        (0, 8): 2,
        (0, 16): 4,
        (2, 8): 4,
        (2, 16): 8,
        (3, 1): 4,
        (3, 2): 4,
        (3, 4): 4,
        (3, 8): 4,
        (4, 8): 2,
        (4, 16): 4,
        (6, 8): 4,
        (6, 16): 8,
    }.get((color_type, bit_depth))
    if decoded_bytes_per_pixel is None:
        raise ValidationFailure("MI2-06 fixture PNG pixel format is unsupported")
    decoded_bytes = pixel_width * pixel_height * decoded_bytes_per_pixel

    blocks = package.get("document", {}).get("blocks")
    if not isinstance(blocks, list):
        raise ValidationFailure("MI2-06 document blocks are missing")
    source_figures = [
        (ordinal, block)
        for ordinal, block in enumerate(blocks)
        if isinstance(block, dict) and block.get("kind") == "figure"
    ]
    figures = selected.get("figures")
    if len(source_figures) != 1 or not isinstance(figures, list) or len(figures) != 1:
        raise ValidationFailure("MI2-06 source/selected figure closure is not exact")
    document_ordinal, source_figure = source_figures[0]
    figure = figures[0]
    caption = source_figure.get("caption")
    if not isinstance(caption, list) or any(
        block.get("kind") not in {"paragraph", "heading"}
        for block in caption
        if isinstance(block, dict)
    ):
        raise ValidationFailure("MI2-06 caption escaped the closed block subflow")
    caption_owners = [block.get("node_id") for block in caption]
    if any(type(owner) is not int for owner in caption_owners):
        raise ValidationFailure("MI2-06 caption owner is invalid")

    figure_rules = [
        rule
        for rule in package.get("style_sheet", {}).get("rules", [])
        if rule.get("selector") == "figure"
    ]
    if len(figure_rules) != 1:
        raise ValidationFailure("MI2-06 fixture has no single computed figure style")
    style = {
        declaration.get("name"): declaration.get("value", {}).get("value")
        for declaration in figure_rules[0].get("declarations", [])
    }
    width = style.get("width")
    start_indent = style.get("start_indent", 0)
    end_indent = style.get("end_indent", 0)
    keep_caption = style.get("keep_caption", True)
    if (
        type(width) is not int
        or width <= 0
        or type(start_indent) is not int
        or start_indent < 0
        or type(end_indent) is not int
        or end_indent < 0
        or type(keep_caption) is not bool
    ):
        raise ValidationFailure("MI2-06 computed width/indent/keep style is invalid")

    masters = package.get("page_masters", {}).get("masters")
    if not isinstance(masters, list) or len(masters) != 1:
        raise ValidationFailure("MI2-06 fixture must use one non-optional page master")
    master = masters[0]
    body = master.get("body")
    if not isinstance(body, dict) or selected.get("body") != body:
        raise ValidationFailure("MI2-06 selected body does not match the page master")
    if width + start_indent + end_indent > body.get("width", -1):
        raise ValidationFailure("MI2-06 computed figure width exceeds its body")

    quotient, remainder = divmod(width * pixel_height, pixel_width)
    if remainder * 2 > pixel_width or (
        remainder * 2 == pixel_width and quotient % 2 == 1
    ):
        quotient += 1
    expected_height = quotient
    if expected_height <= 0:
        raise ValidationFailure("MI2-06 aspect-ratio rounding produced no height")
    expected_rect = {
        "height": expected_height,
        "width": width,
        "x": body["x"] + start_indent,
        "y": body["y"] + expectation["scenarios"]["initial_consumed_block_size"],
    }
    expected_figure_fields = {
        "admitted_byte_length": len(png_bytes),
        "admitted_sha256": declaration["expected_sha256"],
        "alt": source_figure.get("alt"),
        "attested_media_kind": "png",
        "decoded_bytes": decoded_bytes,
        "document_ordinal": document_ordinal,
        "draw_image_count": 1,
        "effective_space_before": 0,
        "figure_flow_id": 0,
        "figure_node_id": source_figure.get("node_id"),
        "image_id": declaration.get("image_id"),
        "keep_policy": (
            "keep_image_and_caption" if keep_caption else "allow_caption_split"
        ),
        "moved_to_fresh_page": False,
        "oversize_policy": "terminal_once",
        "page_index": 0,
        "pixel_height": pixel_height,
        "pixel_width": pixel_width,
        "rect": expected_rect,
    }
    for field, expected in expected_figure_fields.items():
        if figure.get(field) != expected:
            raise ValidationFailure(f"MI2-06 figure {field} is not source/admission-bound")
    caption_flow_id = figure.get("caption_flow_id")
    if type(caption_flow_id) is not int or caption_flow_id <= 0:
        raise ValidationFailure("MI2-06 caption FlowId is not a child flow")

    used = expectation["scenarios"]["initial_consumed_block_size"] + expected_height
    page_index = 0
    expected_fragments: list[dict[str, Any]] = []
    page_counts = [{"caption_block_count": 0, "figure_count": 1, "page_index": 0}]
    caption_sizes = expectation["scenarios"]["caption_block_sizes"]
    if len(caption_sizes) != len(caption_owners):
        raise ValidationFailure("MI2-06 caption measurement closure drifted")
    for owner, block_size in zip(caption_owners, caption_sizes, strict=True):
        if block_size > body["height"]:
            raise ValidationFailure("MI2-06 checked fixture unexpectedly oversizes a caption")
        if used + block_size > body["height"]:
            page_index += 1
            used = 0
            page_counts.append(
                {"caption_block_count": 0, "figure_count": 0, "page_index": page_index}
            )
        expected_fragments.append(
            {
                "caption_flow_id": caption_flow_id,
                "caption_node_id": owner,
                "page_index": page_index,
                "rect": {
                    "height": block_size,
                    "width": body["width"],
                    "x": body["x"],
                    "y": body["y"] + used,
                },
            }
        )
        used += block_size
        page_counts[page_index]["caption_block_count"] += 1
    if figure.get("caption_fragments") != expected_fragments:
        raise ValidationFailure("MI2-06 caption FlowId/owner/page geometry closure is broken")
    if selected.get("pages") != page_counts or selected.get("page_count") != len(page_counts):
        raise ValidationFailure("MI2-06 selected/PDF page closure is broken")
    if (
        selected.get("page_width") != master.get("width")
        or selected.get("page_height") != master.get("height")
        or selected.get("master_id") != master.get("master_id")
    ):
        raise ValidationFailure("MI2-06 selected page geometry is not master-bound")

    package_digest = hashlib.sha256(jcs_bytes(package)).hexdigest()
    if selected.get("package_sha256") != package_digest:
        raise ValidationFailure("MI2-06 package fingerprint does not bind canonical bytes")
    if selected.get("layout_state_sha256") != selected.get("selected_state_sha256"):
        raise ValidationFailure("MI2-06 selected/display layout state closure is broken")
    expected_xobjects = [
        {"image_id": declaration["image_id"], "resource_name": "/Im0"}
    ]
    if selected.get("image_xobjects") != expected_xobjects:
        raise ValidationFailure("MI2-06 logical image/PDF resource-name closure is broken")
    if selected.get("image_xobject_count") != expectation["scenarios"][
        "image_xobject_count"
    ]:
        raise ValidationFailure("MI2-06 serialized image-XObject count drifted")
    if selected["image_xobject_count"] < len(expected_xobjects):
        raise ValidationFailure("MI2-06 serialized image-XObject closure is incomplete")


def validate_staging_machine_link_bundle(
    package: dict[str, Any],
    selected: dict[str, Any],
    expectation: dict[str, Any],
    font_bytes: bytes,
) -> None:
    """Close MI2-07 from package link children through serialized annotations."""

    expected_expectation = {
        "contract": "typaxis.contract/1.2",
        "profile": "typaxis.machine-pdf/basic-document-1",
        "policy_version": "typaxis.basic-link-policy/1",
        "scenarios": {
            "annotation_count": 3,
            "annotation_closure": [
                "missing",
                "extra",
                "wrong_page",
                "wrong_target",
            ],
            "bad_target": "preflight",
            "bad_uri": "preflight",
            "destination_count": 1,
            "deterministic_double_build": True,
            "empty_children": "preflight",
            "exact_rectangle_limit": 3,
            "external_uri": "https://example.test/Path?Q=1",
            "internal_anchor": "target",
            "public_rejected": True,
            "rectangle_tamper": "closure",
            "unpainted_children": "preflight",
            "wrapped_external_rectangles": 2,
        },
    }
    if expectation != expected_expectation:
        raise ValidationFailure("MI2-07 scenario expectation drifted")
    if (
        selected.get("contract") != expectation["contract"]
        or selected.get("profile") != expectation["profile"]
        or selected.get("policy_version") != expectation["policy_version"]
    ):
        raise ValidationFailure("MI2-07 contract/profile/policy closure drifted")

    font_faces = package.get("resources", {}).get("font_faces")
    if not isinstance(font_faces, list) or len(font_faces) != 1:
        raise ValidationFailure("MI2-07 fixture must declare exactly one font")
    font_face = font_faces[0]
    if font_face.get("uri") != "body.ttf":
        raise ValidationFailure("MI2-07 font resource path drifted")
    if hashlib.sha256(font_bytes).hexdigest() != font_face.get("expected_sha256"):
        raise ValidationFailure("MI2-07 admitted font hash does not match its declaration")

    blocks = package.get("document", {}).get("blocks")
    if (
        not isinstance(blocks, list)
        or len(blocks) != 1
        or blocks[0].get("kind") != "paragraph"
    ):
        raise ValidationFailure("MI2-07 fixture escaped its closed paragraph domain")
    paragraph = blocks[0]
    paragraph_owner = paragraph.get("node_id")
    children = paragraph.get("children")
    if not isinstance(children, list):
        raise ValidationFailure("MI2-07 paragraph children are missing")
    source_anchors = {
        child.get("anchor_id"): child.get("node_id")
        for child in children
        if isinstance(child, dict) and child.get("kind") == "anchor"
    }
    if source_anchors != {expectation["scenarios"]["internal_anchor"]: 2}:
        raise ValidationFailure("MI2-07 source anchor registry is missing or duplicated")
    source_links = [
        child
        for child in children
        if isinstance(child, dict) and child.get("kind") == "link"
    ]
    selected_links = selected.get("links")
    if (
        len(source_links) != 2
        or not isinstance(selected_links, list)
        or len(selected_links) != len(source_links)
    ):
        raise ValidationFailure("MI2-07 source/selected link closure is not exact")
    if [link.get("link_node_id") for link in selected_links] != sorted(
        link.get("node_id") for link in source_links
    ):
        raise ValidationFailure("MI2-07 links are missing, extra, or noncanonical")

    pages = selected.get("pages")
    page_count = selected.get("page_count")
    if (
        not isinstance(pages, list)
        or type(page_count) is not int
        or page_count != len(pages)
        or [page.get("page_index") for page in pages] != list(range(page_count))
    ):
        raise ValidationFailure("MI2-07 selected/PDF page closure is broken")
    page_by_index = {page["page_index"]: page for page in pages}
    object_count = selected.get("object_count")
    if type(object_count) is not int or object_count <= 0:
        raise ValidationFailure("MI2-07 PDF object count is invalid")

    annotation_ids: list[int] = []
    annotations_per_page = [0] * page_count
    prior_logical_end = 0
    for source_link, link in zip(source_links, selected_links, strict=True):
        owner = source_link.get("node_id")
        if (
            link.get("link_node_id") != owner
            or link.get("paragraph_node_id") != paragraph_owner
        ):
            raise ValidationFailure("MI2-07 link owner/paragraph binding drifted")
        link_children = source_link.get("children")
        if not isinstance(link_children, list) or not link_children:
            raise ValidationFailure("MI2-07 accepted an empty link")
        painted_ranges = [
            child.get("text_span")
            for child in link_children
            if isinstance(child, dict)
            and child.get("kind") == "text"
            and isinstance(child.get("text_span"), dict)
            and child["text_span"].get("end_byte", 0)
            > child["text_span"].get("start_byte", 0)
        ]
        if not painted_ranges:
            raise ValidationFailure("MI2-07 accepted an unpainted link")
        expected_cluster_count = sum(
            span["end_byte"] - span["start_byte"] for span in painted_ranges
        )
        logical_start = link.get("logical_cluster_start")
        logical_end = link.get("logical_cluster_end")
        logical_count = link.get("logical_cluster_count")
        if (
            logical_start != prior_logical_end
            or logical_count != expected_cluster_count
            or logical_end != logical_start + logical_count
        ):
            raise ValidationFailure("MI2-07 logical cluster range is not exact and contiguous")
        prior_logical_end = logical_end

        source_target = source_link.get("target")
        target = link.get("target")
        if not isinstance(source_target, dict) or not isinstance(target, dict):
            raise ValidationFailure("MI2-07 link target is missing")
        if source_target.get("kind") == "internal":
            anchor_id = source_target.get("anchor_id")
            expected_target = {
                "anchor_id": anchor_id,
                "anchor_owner_node_id": source_anchors.get(anchor_id),
                "kind": "internal",
            }
            if target != expected_target:
                raise ValidationFailure("MI2-07 internal target is not anchor-registry-bound")
        elif source_target.get("kind") == "uri":
            raw_uri = source_target.get("uri")
            if not isinstance(raw_uri, str) or ":" not in raw_uri:
                raise ValidationFailure("MI2-07 external target is not a URI")
            scheme, remainder = raw_uri.split(":", 1)
            normalized = f"{scheme.lower()}:{remainder}"
            if scheme.lower() not in {"http", "https", "mailto", "tel"}:
                raise ValidationFailure("MI2-07 external target bypassed the scheme allowlist")
            if target != {"kind": "external", "uri": normalized}:
                raise ValidationFailure("MI2-07 raw URI escaped SafeUri normalization")
        else:
            raise ValidationFailure("MI2-07 source link target kind is unsupported")

        rectangles = link.get("rectangles")
        if not isinstance(rectangles, list) or not rectangles:
            raise ValidationFailure("MI2-07 link has no selected annotation rectangle")
        rectangle_order = [
            (rectangle.get("page_index"), rectangle.get("line_ordinal"))
            for rectangle in rectangles
        ]
        if rectangle_order != sorted(rectangle_order) or len(set(rectangle_order)) != len(
            rectangle_order
        ):
            raise ValidationFailure("MI2-07 rectangle order is noncanonical or duplicated")
        for rectangle in rectangles:
            page_index = rectangle.get("page_index")
            page = page_by_index.get(page_index)
            rect = rectangle.get("rect")
            object_id = rectangle.get("annotation_object_id")
            if page is None or not isinstance(rect, dict):
                raise ValidationFailure("MI2-07 annotation refers to the wrong page")
            if type(object_id) is not int or not 0 < object_id <= object_count:
                raise ValidationFailure("MI2-07 annotation object binding is invalid")
            annotation_ids.append(object_id)
            annotations_per_page[page_index] += 1
            x = rect.get("x")
            y = rect.get("y")
            width = rect.get("width")
            height = rect.get("height")
            if (
                any(type(value) is not int for value in (x, y, width, height))
                or x < 0
                or y < 0
                or width <= 0
                or height <= 0
                or x + width > page.get("width", -1)
                or y + height > page.get("height", -1)
            ):
                raise ValidationFailure("MI2-07 annotation rectangle is empty or out of bounds")

    if len(annotation_ids) != len(set(annotation_ids)):
        raise ValidationFailure("MI2-07 annotation objects are duplicated")
    annotation_count = len(annotation_ids)
    if (
        selected.get("annotation_count") != annotation_count
        or [page.get("annotation_count") for page in pages] != annotations_per_page
        or annotation_count != expectation["scenarios"]["annotation_count"]
        or annotation_count != expectation["scenarios"]["exact_rectangle_limit"]
    ):
        raise ValidationFailure("MI2-07 annotation count closure drifted")

    destinations = selected.get("destinations")
    expected_destinations = [
        {"anchor_id": anchor_id, "owner_node_id": owner}
        for anchor_id, owner in sorted(source_anchors.items(), key=lambda item: utf8_sort_key(item[0]))
    ]
    if not isinstance(destinations, list) or len(destinations) != len(expected_destinations):
        raise ValidationFailure("MI2-07 named destination closure is not exact")
    for destination, expected in zip(destinations, expected_destinations, strict=True):
        if {
            "anchor_id": destination.get("anchor_id"),
            "owner_node_id": destination.get("owner_node_id"),
        } != expected:
            raise ValidationFailure("MI2-07 named destination owner is wrong-package or stale")
        page = page_by_index.get(destination.get("page_index"))
        point = destination.get("point")
        if (
            page is None
            or not isinstance(point, dict)
            or not 0 <= point.get("x", -1) <= page.get("width", -1)
            or not 0 <= point.get("y", -1) <= page.get("height", -1)
        ):
            raise ValidationFailure("MI2-07 named destination is outside its selected page")
    if (
        selected.get("destination_count") != len(destinations)
        or len(destinations) != expectation["scenarios"]["destination_count"]
    ):
        raise ValidationFailure("MI2-07 destination count closure drifted")
    if (
        len(selected_links[1]["rectangles"])
        != expectation["scenarios"]["wrapped_external_rectangles"]
    ):
        raise ValidationFailure("MI2-07 wrapped external link rectangle coverage drifted")
    if selected_links[1]["target"].get("uri") != expectation["scenarios"]["external_uri"]:
        raise ValidationFailure("MI2-07 expected normalized URI drifted")
    if selected.get("package_sha256") != hashlib.sha256(jcs_bytes(package)).hexdigest():
        raise ValidationFailure("MI2-07 package fingerprint does not bind canonical bytes")


def machine_advertised_items(
    capabilities: dict[str, Any], profile_id: str
) -> list[str]:
    profiles = capabilities["machine_input"]["profiles"]
    matches = [profile for profile in profiles if profile.get("id") == profile_id]
    if len(matches) != 1:
        raise ValidationFailure(f"machine capabilities do not contain one {profile_id} profile")
    profile = matches[0]
    items = {
        "source_closure:entry_only",
        "page_master:default",
    }
    items.update(f"block:{item}" for item in profile["blocks"])
    items.update(f"font_format:{item}" for item in profile["font_formats"])
    items.update(f"image_format:{item}" for item in profile["image_formats"])
    items.update(f"inline:{item}" for item in profile["inlines"]["kinds"])
    items.update(
        f"reference_format:{item}" for item in profile["inlines"]["reference_formats"]
    )
    items.update(f"page_value:{item}" for item in profile["page_values"])
    items.update(f"pdf_feature:{item}" for item in profile["pdf_features"])
    items.update(f"style_block_type:{item}" for item in profile["style_block_types"])
    items.update(f"style_property:{item}" for item in profile["style_properties"])
    items.update(f"style_selector:{item}" for item in profile["style_selectors"])
    items.update(
        f"page_frame:{item}" for item in profile["page_master"]["optional_frames"]
    )
    advanced = profile.get("advanced_pagination")
    if isinstance(advanced, dict):
        balance = advanced["balance"]
        items.add(f"advanced_balance:{balance}")
        column_count = advanced["column_count"]
        if column_count is None:
            items.add("advanced_column_count:none")
        else:
            items.add(
                "advanced_column_count:"
                f"{column_count['minimum']}-{column_count['maximum']}"
            )
        items.add(
            f"advanced_custom_trim:{str(advanced['custom_trim']).lower()}"
        )
        items.add(
            f"advanced_header_footer:{str(advanced['header_footer']).lower()}"
        )
        items.update(
            f"advanced_float_class:{item}" for item in advanced["float_classes"]
        )
        items.update(
            f"advanced_master_selection:{item}"
            for item in advanced["master_selection"]
        )
        items.update(
            f"advanced_page_box:{item}" for item in advanced["page_boxes"]
        )
        items.add(f"advanced_page_progression:{advanced['page_progression']}")
        items.add(f"advanced_writing_mode:{advanced['writing_mode']}")
    return sorted(items, key=utf8_sort_key)


def combined_fixture_items(
    package: dict[str, Any], fixture_root: Path, profile_id: str
) -> list[str]:
    items = {"source_closure:entry_only", "page_master:default"}
    saw_anchor = False
    saw_figure = False
    saw_link = False
    saw_text = False
    stack = list(reversed(package["document"]["blocks"]))
    for definition in reversed(package["document"].get("footnotes", [])):
        stack.extend(reversed(definition.get("blocks", [])))
    while stack:
        node = stack.pop()
        kind = node.get("kind")
        if kind in {"figure", "heading", "list", "page_break", "paragraph", "table"}:
            items.add(f"block:{kind}")
        if kind in {
            "anchor",
            "footnote_reference",
            "hard_break",
            "link",
            "reference",
            "soft_break",
            "text",
        }:
            items.add(f"inline:{kind}")
        if kind == "anchor":
            saw_anchor = True
        if kind == "text":
            saw_text = True
        if kind == "figure":
            saw_figure = True
            stack.extend(reversed(node.get("caption", [])))
        if kind == "list":
            for item in reversed(node.get("items", [])):
                stack.extend(reversed(item.get("blocks", [])))
        if kind == "table":
            rows = [*node.get("head", []), *node.get("body", [])]
            for row in reversed(rows):
                for cell in reversed(row.get("cells", [])):
                    stack.extend(reversed(cell.get("blocks", [])))
        if kind == "link":
            saw_link = True
        if kind == "reference":
            items.add(f"reference_format:{node['format']}")
        children = node.get("children")
        if isinstance(children, list):
            stack.extend(reversed(children))
    if saw_anchor:
        items.add("pdf_feature:named-destinations")
    if saw_text:
        items.add("pdf_feature:text-extraction")
    if saw_link:
        items.add("pdf_feature:link-annotations")

    for rule in package["style_sheet"]["rules"]:
        selector = rule["selector"].split(".", 1)[0]
        items.add(f"style_block_type:{selector}")
        items.add(f"style_selector:{selector}")
        for declaration in rule["declarations"]:
            name = declaration["name"]
            items.add(f"style_property:{name}")
            if name == "page" and declaration["value"] == {
                "kind": "keyword",
                "value": "auto",
            }:
                items.add("page_value:auto")
    for face in package["resources"]["font_faces"]:
        payload = (fixture_root / face["uri"]).read_bytes()
        if payload.startswith(b"ttcf"):
            items.add("font_format:ttc-truetype-glyf")
        elif payload.startswith(b"\x00\x01\x00\x00"):
            items.add("font_format:sfnt-truetype-glyf")
        else:
            raise ValidationFailure(f"combined fixture font has an unknown container: {face['uri']}")
    for image in package["resources"]["images"]:
        payload = (fixture_root / image["uri"]).read_bytes()
        if not payload.startswith(b"\x89PNG\r\n\x1a\n"):
            raise ValidationFailure(f"combined fixture image is not PNG: {image['uri']}")
        items.add("image_format:png")
        if saw_figure:
            items.add("pdf_feature:png-xobjects")
    page_masters = package["page_masters"]
    if any(master.get("footnote") is not None for master in page_masters["masters"]):
        items.add("page_frame:footnote")
    advanced_profiles = {
        "typaxis.machine-pdf/columns-1",
        "typaxis.machine-pdf/float-1",
        "typaxis.machine-pdf/header-footer-1",
    }
    if profile_id in advanced_profiles:
        masters = page_masters["masters"]
        items.add(f"advanced_page_progression:{page_masters['page_progression']}")
        items.add(f"advanced_writing_mode:{page_masters['writing_mode']}")
        items.update(
            f"advanced_page_box:{box}" for box in ("crop", "media", "trim")
        )
        custom_trim = any(
            master["trim"]
            != {
                "x": 0,
                "y": 0,
                "width": master["width"],
                "height": master["height"],
            }
            for master in masters
        )
        items.add(f"advanced_custom_trim:{str(custom_trim).lower()}")
        header_footer = any(
            master["header_content"] is not None
            or master["footer_content"] is not None
            for master in masters
        )
        items.add(f"advanced_header_footer:{str(header_footer).lower()}")
        layouts = [
            master["column_layout"]
            for master in masters
            if master["column_layout"] is not None
        ]
        if layouts:
            items.add("advanced_column_count:1-65535")
            balances = {layout["balance"] for layout in layouts}
            items.update(f"advanced_balance:{balance}" for balance in balances)
        else:
            items.add("advanced_column_count:none")
            items.add("advanced_balance:forbidden")
        items.add("advanced_master_selection:single")
        if (
            len(masters) >= 3
            and any(rule.get("first") is True for rule in page_masters["selection_rules"])
            and any(
                rule.get("parity") == "even"
                for rule in page_masters["selection_rules"]
            )
        ):
            items.add("advanced_master_selection:first_left_right")
        if any(
            block.get("kind") == "figure" and block.get("placement") == "float"
            for block in package["document"]["blocks"]
        ):
            items.update(
                f"advanced_float_class:{value}"
                for value in ("here", "top", "bottom", "next_page")
            )
    return sorted(items, key=utf8_sort_key)


def validate_machine_fixture_bundle(validators: dict[str, Draft202012Validator]) -> tuple[int, int]:
    expectation_validator = validators["machine-fixture-expectation.schema.json"]
    matrix_validator = validators["machine-fixture-matrix.schema.json"]
    capabilities_validator = validators["machine-capabilities.schema.json"]

    capabilities_path = MACHINE_FIXTURE_DIR / "capabilities.json"
    capabilities = load_json(capabilities_path)
    errors = schema_errors(capabilities_validator, capabilities)
    if errors:
        raise ValidationFailure(
            f"{capabilities_path}: capabilities snapshot rejected: " + " | ".join(errors)
        )
    if capabilities_path.read_bytes() != jcs_bytes(capabilities):
        raise ValidationFailure(f"{capabilities_path}: capabilities snapshot is not canonical JCS")
    conformance_capabilities = load_json(CONFORMANCE_DIR / "machine-capabilities.json")
    if capabilities != conformance_capabilities:
        raise ValidationFailure("machine fixture capability snapshot differs from conformance")

    expectations: dict[str, tuple[Path, dict[str, Any]]] = {}
    expected_paths = sorted(MACHINE_FIXTURE_DIR.glob("**/expected.json"))
    if not expected_paths:
        raise ValidationFailure("machine fixture bundle contains no expected.json files")
    for path in expected_paths:
        instance = load_json(path)
        errors = schema_errors(expectation_validator, instance)
        if errors:
            raise ValidationFailure(f"{path}: expectation rejected: " + " | ".join(errors))
        if path.read_bytes() != jcs_bytes(instance):
            raise ValidationFailure(f"{path}: expectation is not canonical JCS")
        fixture_id = instance["fixture_id"]
        if fixture_id in expectations:
            raise ValidationFailure(f"machine fixture ID is duplicated: {fixture_id}")
        expectations[fixture_id] = (path, instance)
        coverage = instance["advertised_item_coverage"]
        if coverage != sorted(coverage, key=utf8_sort_key):
            raise ValidationFailure(f"{path}: advertised coverage is not canonical")
        package = path.parent / instance["package"]
        if not package.is_file():
            raise ValidationFailure(f"{path}: PACKAGE does not exist: {package}")
        outcome = instance["expected"]
        visible = set(outcome["visible_artifacts"])
        if outcome["exit_code"] == 0:
            if (
                outcome["primary_code"] is not None
                or outcome["location"] is not None
                or outcome["page_count"] is None
                or "pdf" not in visible
            ):
                raise ValidationFailure(f"{path}: success outcome fields are inconsistent")
        elif (
            outcome["page_count"] is not None
            or outcome["normalized_extracted_text"] is not None
            or "pdf" in visible
        ):
            raise ValidationFailure(f"{path}: failure outcome claims a PDF result")
        if outcome["exit_code"] == 2 and visible:
            raise ValidationFailure(f"{path}: usage failure claims visible artifacts")
        for index, record in enumerate(instance["resource_hashes"]):
            verify_file_record(path.parent / "job", record, f"{path} resource {index}")

        try:
            package_value = load_json(package)
        except ValidationFailure:
            package_value = None
        if isinstance(package_value, dict) and "contract" in package_value:
            if package_value["contract"] != instance["contract"]:
                raise ValidationFailure(f"{path}: expected contract differs from PACKAGE")

    combined_profiles = (
        (
            "typaxis.machine-pdf/paragraph-1",
            "paragraph-1.combined",
            "Typaxis machine input",
        ),
        (
            "typaxis.machine-pdf/basic-document-1",
            "basic-document-1.combined",
            "Basic document internal external First item Second entry PNG caption",
        ),
        (
            "typaxis.machine-pdf/table-1",
            "table-1.combined",
            "Basic document internal external First item Second entry PNG caption Header A Header B alpha beta Header A delta Header B gamma",
        ),
        (
            "typaxis.machine-pdf/footnote-1",
            "footnote-1.combined",
            "Basic document internal external First item Second entry Z first Z second A note A tail PNG caption Z third Z fourth Z fifth",
        ),
        (
            "typaxis.machine-pdf/columns-1",
            "columns-1.combined",
            "Basic document internal external First item Second entry PNG caption",
        ),
        (
            "typaxis.machine-pdf/float-1",
            "float-1.combined",
            "Basic document internal external First item Second entry PNG caption",
        ),
        (
            "typaxis.machine-pdf/header-footer-1",
            "header-footer-1.combined",
            "First header Basic document internal external First item Second entry First footer Left header PNG caption Left footer Right header Right footer",
        ),
    )
    for profile_id, fixture_id, expected_text in combined_profiles:
        advertised = machine_advertised_items(capabilities, profile_id)
        combined_path, combined = expectations[fixture_id]
        if combined["advertised_item_coverage"] != advertised:
            raise ValidationFailure(
                f"{fixture_id} does not cover the exact advertised descriptor"
            )
        combined_package = load_json(combined_path.parent / combined["package"])
        observed_items = combined_fixture_items(
            combined_package, combined_path.parent / "job", profile_id
        )
        if observed_items != advertised:
            raise ValidationFailure(
                f"{fixture_id} PACKAGE coverage differs from capabilities: "
                f"missing={sorted(set(advertised) - set(observed_items))}, "
                f"extra={sorted(set(observed_items) - set(advertised))}"
            )
        if combined["expected"]["normalized_extracted_text"] != expected_text:
            raise ValidationFailure(f"{fixture_id} normalized extracted text is not fixed")

    table_combined_path, table_combined = expectations["table-1.combined"]
    table_package = load_json(table_combined_path.parent / table_combined["package"])
    direct_tables = [
        block
        for block in table_package["document"]["blocks"]
        if block.get("kind") == "table"
    ]
    if len(direct_tables) != 1:
        raise ValidationFailure("table-1.combined must contain one direct-body table")
    table = direct_tables[0]
    column_kinds = [column.get("kind") for column in table.get("columns", [])]
    cells = [
        cell
        for row in [*table.get("head", []), *table.get("body", [])]
        for cell in row.get("cells", [])
    ]
    if (
        "fixed" not in column_kinds
        or "fraction" not in column_kinds
        or not table.get("head")
        or not table.get("body")
        or not any(cell.get("colspan", 0) > 1 for cell in cells)
        or not any(cell.get("rowspan", 0) > 1 for cell in cells)
        or table_combined["expected"]["page_count"] < 2
        or table_combined["expected"]["normalized_extracted_text"].count("Header A") < 2
    ):
        raise ValidationFailure(
            "table-1.combined lacks fixed/fraction, colspan/rowspan, or repeated-header coverage"
        )

    machine_test_source = (
        REPOSITORY_ROOT / "workspace/crates/typaxis-cli/src/machine_tests.rs"
    ).read_text("utf-8")
    machine_test_names = set(
        re.findall(r"^\s*fn\s+([a-z0-9_]+)\s*\(\s*\)", machine_test_source, re.MULTILINE)
    )
    verification_commands = [
        "cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine --locked",
        "cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package --locked",
        "python3 schemas/validate.py",
    ]
    required_decoder_rows = {
        "m1-decoder-01": ("machine_tests::matrix_01_blank_1_1", ("paragraph-1.blank-1.1",)),
        "m1-decoder-02": ("machine_tests::matrix_02_blank_1_0", ("paragraph-1.blank-1.0",)),
        "m1-decoder-03": ("machine_tests::matrix_03_combined", ("paragraph-1.combined",)),
        "m1-decoder-04": (
            "machine_tests::matrix_04_package_envelope",
            ("p1100.bom", "p1100.nul", "p1100.trailing.token"),
        ),
        "m1-decoder-05": (
            "machine_tests::matrix_05_json_grammar",
            ("p1101.malformed.json", "p1101.duplicate.escaped.key"),
        ),
        "m1-decoder-06": (
            "machine_tests::matrix_06_typed_members",
            (
                "p1102.unknown.field",
                "p1102.missing.field",
                "p1102.float.integer",
                "p1102.range",
            ),
        ),
        "m1-decoder-07": (
            "machine_tests::matrix_07_unknown_contract",
            ("p1103.unknown-contract",),
        ),
        "m1-decoder-08": (
            "machine_tests::matrix_08_package_bytes",
            ("i9100.package-bytes-exact", "i9100.package-bytes-max-plus-one"),
        ),
        "m1-decoder-09": (
            "machine_tests::matrix_09_json_depth",
            ("i9101.depth-exact", "i9101.depth-max-plus-one"),
        ),
        "m1-decoder-10": (
            "machine_tests::matrix_10_source_profile",
            ("p1110.multiple-sources", "p1110.nonzero-entry"),
        ),
        "m1-decoder-11": (
            "machine_tests::matrix_11_source_path",
            ("p1111.unsafe-source", "i9112.source-symlink"),
        ),
        "m1-decoder-12": (
            "machine_tests::matrix_12_package_root",
            ("usage.package-outside-root",),
        ),
        "m1-decoder-13": (
            "machine_tests::matrix_13_package_open",
            ("i9111.package-symlink",),
        ),
        "m1-decoder-14": (
            "machine_tests::matrix_14_source_identity",
            ("p1112.source-length", "p1112.source-hash"),
        ),
        "m1-decoder-15": (
            "machine_tests::matrix_15_stable_read",
            ("i9113.package-mutation", "i9113.source-mutation"),
        ),
        "m1-decoder-16": (
            "machine_tests::matrix_16_identity_map",
            ("p1112.identity-map",),
        ),
        "m1-decoder-17": (
            "machine_tests::matrix_17_unsupported_content",
            ("l5100.unsupported.content",),
        ),
        "m1-decoder-18": (
            "machine_tests::matrix_18_unsupported_style",
            ("l5101.unsupported.style.master",),
        ),
        "m1-decoder-19": (
            "machine_tests::matrix_19_unsupported_image",
            ("r7100.unsupported.image",),
        ),
        "m1-decoder-20": (
            "machine_tests::matrix_20_host_unavailable",
            ("i9110.host-unavailable",),
        ),
        "m1-decoder-21": (
            "machine_tests::matrix_21_unknown_profile",
            ("usage.unknown-profile",),
        ),
        "m1-decoder-22": (
            "machine_tests::matrix_22_blank_1_2",
            ("paragraph-1.blank-1.2",),
        ),
    }

    listed_paths: set[Path] = set()
    listed_tests: set[str] = set()
    matrix_paths = sorted((MACHINE_FIXTURE_DIR / "matrices").glob("*.json"))
    observed_decoder_rows: dict[str, tuple[str, tuple[str, ...]]] = {}
    for path in matrix_paths:
        matrix = load_json(path)
        is_m3_aggregate = path.name == "m3-all.json"
        errors = schema_errors(matrix_validator, matrix)
        if errors:
            raise ValidationFailure(f"{path}: matrix rejected: " + " | ".join(errors))
        if path.read_bytes() != jcs_bytes(matrix):
            raise ValidationFailure(f"{path}: matrix is not canonical JCS")
        if matrix["verification_commands"] != verification_commands:
            raise ValidationFailure(f"{path}: verification commands differ from the MI1-16 gate")
        fixture_entries = matrix["fixtures"]
        fixture_ids = [entry["fixture_id"] for entry in fixture_entries]
        if len(fixture_ids) != len(set(fixture_ids)):
            raise ValidationFailure(f"{path}: fixture ID is listed more than once")
        if fixture_ids != sorted(fixture_ids, key=utf8_sort_key):
            raise ValidationFailure(f"{path}: fixture entries are not in canonical ID order")
        entry_ids = set(fixture_ids)
        if is_m3_aggregate:
            required_m3_combined = {
                "columns-1.combined",
                "float-1.combined",
                "footnote-1.combined",
                "header-footer-1.combined",
                "table-1.combined",
            }
            if (
                matrix["profile"] != "typaxis.machine-pdf/m3-all"
                or entry_ids != required_m3_combined
            ):
                raise ValidationFailure(
                    f"{path}: aggregate M3 matrix differs from the complete public set"
                )
        row_ids = [row["id"] for row in matrix["rows"]]
        tests = [row["test"] for row in matrix["rows"]]
        if len(row_ids) != len(set(row_ids)) or len(tests) != len(set(tests)):
            raise ValidationFailure(f"{path}: row IDs and test names must be unique")
        for test in tests:
            if test in listed_tests:
                raise ValidationFailure(f"{path}: test is duplicated across matrices: {test}")
            listed_tests.add(test)
            prefix, separator, test_name = test.partition("::")
            if separator != "::" or prefix != "machine_tests" or test_name not in machine_test_names:
                raise ValidationFailure(f"{path}: matrix test does not exist: {test}")
        row_fixture_ids = [item for row in matrix["rows"] for item in row["fixture_ids"]]
        if len(row_fixture_ids) != len(set(row_fixture_ids)) or set(row_fixture_ids) != entry_ids:
            raise ValidationFailure(f"{path}: each listed fixture must map to exactly one row")
        for row in matrix["rows"]:
            if row["id"].startswith("m1-decoder-"):
                observed_decoder_rows[row["id"]] = (
                    row["test"],
                    tuple(row["fixture_ids"]),
                )
        for entry in fixture_entries:
            fixture_id = entry["fixture_id"]
            expected_path = MACHINE_FIXTURE_DIR / entry["expected"]
            expected = expectations.get(fixture_id)
            if expected is None or expected[0] != expected_path:
                raise ValidationFailure(f"{path}: fixture path/ID mismatch for {fixture_id}")
            if not is_m3_aggregate and expected[1]["profile"] != matrix["profile"]:
                raise ValidationFailure(f"{path}: profile mismatch for {fixture_id}")
            if not is_m3_aggregate and expected_path in listed_paths:
                raise ValidationFailure(f"{path}: expectation is duplicated across matrices")
            if not is_m3_aggregate:
                listed_paths.add(expected_path)
    if observed_decoder_rows != required_decoder_rows:
        raise ValidationFailure(
            "M1 decoder matrix rows differ from docs/25 §15.1: "
            f"expected={required_decoder_rows}, observed={observed_decoder_rows}"
        )
    if listed_paths != set(expected_paths):
        raise ValidationFailure(
            "machine matrices do not cover every expectation exactly once: "
            f"missing={sorted(str(path) for path in set(expected_paths) - listed_paths)}"
        )
    return len(expected_paths), len(matrix_paths)


_GRANDFATHERED_LANGUAGE_TAGS = {
    value.lower(): value
    for value in (
        "art-lojban", "cel-gaulish", "en-GB-oed", "i-ami", "i-bnn",
        "i-default", "i-enochian", "i-hak", "i-klingon", "i-lux",
        "i-mingo", "i-navajo", "i-pwn", "i-tao", "i-tay", "i-tsu",
        "no-bok", "no-nyn", "sgn-BE-FR", "sgn-BE-NL", "sgn-CH-DE",
        "zh-guoyu", "zh-hakka", "zh-min", "zh-min-nan", "zh-xiang",
    )
}


def canonical_book_language(value: str) -> str:
    """Registry-independent RFC 5646 structural validation for MI4 evidence."""

    if (
        not value
        or len(value.encode("utf-8")) > 255
        or "_" in value
        or any(ord(character) >= 128 for character in value)
    ):
        raise ValidationFailure("book-navigation language syntax is invalid")
    parts = value.split("-")
    if any(
        not part
        or len(part) > 8
        or not all(character.isascii() and character.isalnum() for character in part)
        for part in parts
    ):
        raise ValidationFailure("book-navigation language subtag is invalid")
    lowered = "-".join(part.lower() for part in parts)
    if lowered in _GRANDFATHERED_LANGUAGE_TAGS:
        return _GRANDFATHERED_LANGUAGE_TAGS[lowered]
    if parts[0].lower() == "x":
        if len(parts) < 2:
            raise ValidationFailure("book-navigation private-use language is empty")
        return "-".join(part.lower() for part in parts)

    first = parts[0]
    if not first.isalpha() or not (2 <= len(first) <= 8):
        raise ValidationFailure("book-navigation primary language is invalid")
    index = 1
    output = [first.lower()]
    if len(first) in (2, 3):
        extlang_count = 0
        while (
            index < len(parts)
            and len(parts[index]) == 3
            and parts[index].isalpha()
            and extlang_count < 3
        ):
            output.append(parts[index].lower())
            index += 1
            extlang_count += 1
    if index < len(parts) and len(parts[index]) == 4 and parts[index].isalpha():
        output.append(parts[index][0].upper() + parts[index][1:].lower())
        index += 1
    if index < len(parts) and (
        (len(parts[index]) == 2 and parts[index].isalpha())
        or (len(parts[index]) == 3 and parts[index].isdigit())
    ):
        output.append(parts[index].upper() if parts[index].isalpha() else parts[index])
        index += 1

    variants: list[str] = []
    while index < len(parts):
        part = parts[index]
        if (5 <= len(part) <= 8 and part.isalnum()) or (
            len(part) == 4 and part[0].isdigit() and part[1:].isalnum()
        ):
            variant = part.lower()
            if variant in variants:
                raise ValidationFailure("book-navigation language variant is duplicated")
            variants.append(variant)
            index += 1
        else:
            break
    output.extend(variants)

    extensions: list[tuple[str, list[str]]] = []
    singletons: set[str] = set()
    while index < len(parts) and len(parts[index]) == 1 and parts[index].lower() != "x":
        singleton = parts[index].lower()
        if singleton in singletons:
            raise ValidationFailure("book-navigation language singleton is duplicated")
        singletons.add(singleton)
        index += 1
        values: list[str] = []
        while index < len(parts) and 2 <= len(parts[index]) <= 8:
            values.append(parts[index].lower())
            index += 1
        if not values:
            raise ValidationFailure("book-navigation language extension is empty")
        extensions.append((singleton, values))
    for singleton, values in sorted(extensions):
        output.append(singleton)
        output.extend(values)
    if index < len(parts) and parts[index].lower() == "x":
        index += 1
        if index == len(parts):
            raise ValidationFailure("book-navigation private-use suffix is empty")
        output.append("x")
        output.extend(part.lower() for part in parts[index:])
        index = len(parts)
    if index != len(parts):
        raise ValidationFailure("book-navigation language tail is invalid")
    return "-".join(output)


def _book_metadata_string(value: str, label: str) -> None:
    whitespace = {
        0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x0020, 0x0085,
        0x00A0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000,
        *range(0x2000, 0x200B),
    }
    if (
        not value
        or all(ord(character) in whitespace for character in value)
        or any(
            ord(character) <= 0x1F
            or 0x7F <= ord(character) <= 0x9F
            or ord(character) in (0xFFFE, 0xFFFF)
            for character in value
        )
    ):
        raise ValidationFailure(f"book-navigation {label} is not a valid metadata string")


def validate_book_navigation_semantics(
    document: dict[str, Any], manifest: dict[str, Any] | None = None
) -> None:
    """Validate private MI4 rules that JSON Schema cannot express."""

    metadata = document["metadata"]
    for field in ("author", "identifier", "subject", "title"):
        if metadata[field] is not None:
            _book_metadata_string(metadata[field], f"metadata/{field}")
    previous_keyword: bytes | None = None
    for index, keyword in enumerate(metadata["keywords"]):
        _book_metadata_string(keyword, f"metadata/keywords/{index}")
        encoded = keyword.encode("utf-8")
        if previous_keyword is not None and previous_keyword >= encoded:
            raise ValidationFailure("book-navigation keywords are not strict UTF-8 order")
        previous_keyword = encoded
    for field in ("created", "modified"):
        value = metadata[field]
        if value is None:
            continue
        if not re.fullmatch(
            r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
            value,
        ):
            raise ValidationFailure(f"book-navigation metadata/{field} is not UTC-second")
        try:
            parsed = datetime.datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
        except ValueError as error:
            raise ValidationFailure(
                f"book-navigation metadata/{field} is not Gregorian"
            ) from error
        if parsed.year == 0:
            raise ValidationFailure(f"book-navigation metadata/{field} year is zero")
    if (
        metadata["created"] is not None
        and metadata["modified"] is not None
        and metadata["modified"] < metadata["created"]
    ):
        raise ValidationFailure("book-navigation modified precedes created")

    records: dict[int, dict[str, Any]] = {}
    outline_owners: list[dict[str, Any]] = []
    anchor_owners: dict[str, int] = {}

    def add_anchor_owner(node: dict[str, Any]) -> None:
        anchor_id = node.get("anchor_id")
        if anchor_id is None:
            return
        if anchor_id in anchor_owners:
            raise ValidationFailure("book-navigation AnchorId is duplicated")
        anchor_owners[anchor_id] = node["node_id"]

    def add_language_owner(
        node: dict[str, Any], kind: str, parent_id: int | None, inherited: str | None
    ) -> str:
        explicit = node.get("language")
        effective = canonical_book_language(explicit) if explicit is not None else inherited
        if effective is None:
            raise ValidationFailure("book-navigation document language is absent")
        node_id = node["node_id"]
        if node_id in records:
            raise ValidationFailure("book-navigation language NodeId is duplicated")
        records[node_id] = {
            "effective_language": effective,
            "explicit_language": (
                canonical_book_language(explicit) if explicit is not None else None
            ),
            "logical_parent_node_id": parent_id,
            "node_id": node_id,
            "node_kind": kind,
        }
        return effective

    def visit_inline(node: dict[str, Any], parent_id: int, inherited: str) -> None:
        kind = node["kind"]
        if kind in {"anchor", "soft_break", "hard_break"}:
            if "language" in node:
                raise ValidationFailure("book-navigation language override has no owner")
            if kind == "anchor":
                add_anchor_owner(node)
            return
        effective = add_language_owner(node, kind, parent_id, inherited)
        for child in node.get("children", []):
            visit_inline(child, node["node_id"], effective)

    def visit_block(node: dict[str, Any], parent_id: int, inherited: str) -> None:
        kind = node["kind"]
        if kind == "page_break":
            if "language" in node:
                raise ValidationFailure("book-navigation page break has a language")
            return
        effective = add_language_owner(node, kind, parent_id, inherited)
        if kind in {"heading", "semantic_container"}:
            add_anchor_owner(node)
            outline_owners.append(
                {
                    "anchor_id": node.get("anchor_id"),
                    "heading_level": node.get("level"),
                    "kind": kind,
                    "language": effective,
                    "node_id": node["node_id"],
                }
            )
        if kind in {"paragraph", "heading"}:
            for child in node["children"]:
                visit_inline(child, node["node_id"], effective)
        elif kind == "semantic_container":
            for child in node["blocks"]:
                visit_block(child, node["node_id"], effective)
        elif kind == "list":
            for item in node["items"]:
                item_effective = add_language_owner(
                    item, "list_item", node["node_id"], effective
                )
                for child in item["blocks"]:
                    visit_block(child, item["node_id"], item_effective)
        elif kind == "table":
            for row in [*node["head"], *node["body"]]:
                row_effective = add_language_owner(
                    row, "table_row", node["node_id"], effective
                )
                for cell in row["cells"]:
                    cell_effective = add_language_owner(
                        cell, "table_cell", row["node_id"], row_effective
                    )
                    for child in cell["blocks"]:
                        visit_block(child, cell["node_id"], cell_effective)
        elif kind == "figure":
            for child in node["caption"]:
                visit_block(child, node["node_id"], effective)

    root = document["document"]
    document_language = add_language_owner(root, "document", None, None)
    for block in root["blocks"]:
        visit_block(block, root["node_id"], document_language)
    for footnote in root["footnotes"]:
        footnote_language = add_language_owner(
            footnote, "footnote_definition", root["node_id"], document_language
        )
        for block in footnote["blocks"]:
            visit_block(block, footnote["node_id"], footnote_language)
    for master in document["page_masters"]["masters"]:
        for region_name in ("header_content", "footer_content"):
            region = master[region_name]
            if region is None:
                continue
            for block in region["blocks"]:
                block_language = add_language_owner(
                    block, block["kind"], root["node_id"], document_language
                )
                for inline in block["children"]:
                    if inline["kind"] == "text":
                        add_language_owner(
                            inline, "text", block["node_id"], block_language
                        )
    canonical_records = [records[node_id] for node_id in sorted(records)]

    owner_by_id = {owner["node_id"]: owner for owner in outline_owners}
    owner_order = {owner["node_id"]: index for index, owner in enumerate(outline_owners)}
    previous_owner = -1
    seen_sources: set[int] = set()
    seen_destinations: set[str] = set()
    stack: list[int] = []
    entries = document["outline"]["entries"]
    for index, entry in enumerate(entries):
        if entry["outline_id"] != index:
            raise ValidationFailure("book-navigation outline IDs are not dense")
        level = entry["level"]
        if level > len(stack) + 1:
            raise ValidationFailure("book-navigation outline level jumps")
        stack = stack[: level - 1]
        expected_parent = stack[-1] if stack else None
        if entry["parent_outline_id"] != expected_parent:
            raise ValidationFailure("book-navigation outline parent disagrees with preorder")
        stack.append(index)
        owner = owner_by_id.get(entry["source_node_id"])
        if owner is None or owner["kind"] != entry["source_kind"]:
            raise ValidationFailure("book-navigation outline source owner differs")
        if owner["anchor_id"] != entry["destination"]:
            raise ValidationFailure("book-navigation outline destination differs from owner")
        if anchor_owners.get(entry["destination"]) != entry["source_node_id"]:
            raise ValidationFailure("book-navigation outline anchor owner differs")
        if entry["source_kind"] == "heading" and owner["heading_level"] != level:
            raise ValidationFailure("book-navigation outline heading level differs")
        if entry["source_node_id"] in seen_sources or entry["destination"] in seen_destinations:
            raise ValidationFailure("book-navigation outline source/destination is duplicated")
        if owner_order[entry["source_node_id"]] <= previous_owner:
            raise ValidationFailure("book-navigation outline is not source-owner preorder")
        previous_owner = owner_order[entry["source_node_id"]]
        seen_sources.add(entry["source_node_id"])
        seen_destinations.add(entry["destination"])
        _book_metadata_string(entry["label"], f"outline/{index}/label")

    if manifest is None:
        return
    if manifest["metadata"] != metadata:
        raise ValidationFailure("book-navigation manifest metadata differs from source")
    if manifest["document_language"] != document_language:
        raise ValidationFailure("book-navigation manifest document language differs")
    if manifest["languages"] != canonical_records:
        raise ValidationFailure("book-navigation manifest computed languages differ")
    if len(manifest["outline"]) != len(entries):
        raise ValidationFailure("book-navigation manifest outline coverage differs")
    for source, fact in zip(entries, manifest["outline"], strict=True):
        owner = owner_by_id[source["source_node_id"]]
        for key in (
            "destination", "label", "level", "outline_id",
            "parent_outline_id", "source_node_id",
        ):
            if fact[key] != source[key]:
                raise ValidationFailure("book-navigation manifest outline source differs")
        if (
            fact["source_kind"] != source["source_kind"]
            or fact["source_language"] != owner["language"]
        ):
            raise ValidationFailure("book-navigation manifest outline owner differs")
        if fact["pdf_object_number"] <= 0 or fact["page_index"] < 0:
            raise ValidationFailure("book-navigation manifest selected/PDF target is invalid")
    expected_producer = f'{manifest["engine"]["name"]} {manifest["engine"]["version"]}'
    if manifest["pdf"]["producer"] != expected_producer:
        raise ValidationFailure("book-navigation manifest producer differs")


def main() -> int:
    failures: list[str] = []

    try:
        frozen_schemas, frozen_validators, frozen_reference_count = load_schema_registry(
            FROZEN_SCHEMA_DIR, "1.0"
        )
        previous_schemas, previous_validators, previous_reference_count = load_schema_registry(
            PREVIOUS_SCHEMA_DIR, "1.1"
        )
        schemas, validators, reference_count = load_schema_registry(SCHEMA_DIR, "1.3")
        staging_schemas, staging_validators, staging_reference_count = load_schema_registry(
            FROZEN_1_2_SCHEMA_DIR, "1.2"
        )
        versioned_current_schemas, versioned_current_validators, versioned_current_reference_count = (
            load_schema_registry(VERSIONED_CURRENT_SCHEMA_DIR, "1.3")
        )
        private_m4_schemas, private_m4_validators, private_m4_reference_count = (
            load_schema_registry(PRIVATE_M4_SCHEMA_DIR, "1.4")
        )
        if set(staging_schemas) != {
            "build-manifest.schema.json",
            "common.schema.json",
            "diagnostics.schema.json",
            "display-list.schema.json",
            "document-package.schema.json",
            "layout-trace.schema.json",
            "machine-capabilities.schema.json",
            "machine-block-style-manifest.schema.json",
            "machine-fixture-expectation.schema.json",
            "machine-fixture-matrix.schema.json",
            "machine-forced-page-break-manifest.schema.json",
            "machine-forced-page-break-trace.schema.json",
            "machine-figure-manifest.schema.json",
            "machine-footnote-manifest.schema.json",
            "machine-link-manifest.schema.json",
            "machine-list-manifest.schema.json",
            "machine-profile-evidence.schema.json",
            "machine-table-manifest.schema.json",
            "package-config.schema.json",
        }:
            raise ValidationFailure("the versioned 1.2 registry has a missing or extra schema")
        expected_current_aliases = {
            "build-manifest.schema.json",
            "common.schema.json",
            "diagnostics.schema.json",
            "display-list.schema.json",
            "document-package.schema.json",
            "layout-trace.schema.json",
            "machine-advanced-pagination-manifest.schema.json",
            "machine-capabilities.schema.json",
            "machine-fixture-expectation.schema.json",
            "machine-fixture-matrix.schema.json",
            "machine-footnote-manifest.schema.json",
            "machine-profile-evidence.schema.json",
            "machine-table-manifest.schema.json",
            "package-config.schema.json",
        }
        expected_versioned_current = {
            *set(staging_schemas),
            "machine-advanced-pagination-manifest.schema.json",
        }
        if set(schemas) != expected_current_aliases:
            raise ValidationFailure("the current 1.3 alias registry has a missing or extra schema")
        if set(versioned_current_schemas) != expected_versioned_current:
            raise ValidationFailure("the versioned 1.3 registry has a missing or extra schema")
        expected_private_m4 = {
            *expected_versioned_current,
            "machine-accessibility-manifest.schema.json",
            "machine-book-navigation-manifest.schema.json",
            "machine-math-manifest.schema.json",
            "machine-safe-vector-manifest.schema.json",
            "machine-semantic-container-manifest.schema.json",
        }
        if set(private_m4_schemas) != expected_private_m4:
            raise ValidationFailure("the private 1.4 registry has a missing or extra schema")
        for filename in schemas:
            if (SCHEMA_DIR / filename).read_bytes() != (
                VERSIONED_CURRENT_SCHEMA_DIR / filename
            ).read_bytes():
                raise ValidationFailure(
                    f"the current 1.3 alias differs from its versioned schema: {filename}"
                )
        if set(frozen_schemas) != set(FROZEN_SCHEMA_SHA256):
            raise ValidationFailure("the frozen 1.0 registry has a missing or extra schema")
        for filename, expected_digest in FROZEN_SCHEMA_SHA256.items():
            observed_digest = hashlib.sha256(
                (FROZEN_SCHEMA_DIR / filename).read_bytes()
            ).hexdigest()
            if observed_digest != expected_digest:
                raise ValidationFailure(
                    f"the frozen 1.0 schema bytes changed: {filename}"
                )
        if set(previous_schemas) != set(PREVIOUS_SCHEMA_SHA256):
            raise ValidationFailure("the frozen 1.1 registry has a missing or extra schema")
        for filename, expected_digest in PREVIOUS_SCHEMA_SHA256.items():
            observed_digest = hashlib.sha256(
                (PREVIOUS_SCHEMA_DIR / filename).read_bytes()
            ).hexdigest()
            if observed_digest != expected_digest:
                raise ValidationFailure(
                    f"the frozen 1.1 schema bytes changed: {filename}"
                )
        if set(staging_schemas) != set(FROZEN_1_2_SCHEMA_SHA256):
            raise ValidationFailure("the frozen 1.2 registry has a missing or extra schema")
        for filename, expected_digest in FROZEN_1_2_SCHEMA_SHA256.items():
            observed_digest = hashlib.sha256(
                (FROZEN_1_2_SCHEMA_DIR / filename).read_bytes()
            ).hexdigest()
            if observed_digest != expected_digest:
                raise ValidationFailure(
                    f"the frozen 1.2 schema bytes changed: {filename}"
                )

        semantic_document_path = (
            STAGING_SEMANTIC_CONTAINER_FIXTURE_DIR / "job" / "document-package.json"
        )
        semantic_manifest_path = (
            STAGING_SEMANTIC_CONTAINER_FIXTURE_DIR
            / "staging-semantic-container.json"
        )
        semantic_document = load_json(semantic_document_path)
        semantic_manifest = load_json(semantic_manifest_path)
        semantic_document_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], semantic_document
        )
        if semantic_document_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the semantic-container fixture: "
                + " | ".join(semantic_document_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            semantic_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private 1.4 fixture"
            )
        semantic_manifest_errors = schema_errors(
            private_m4_validators[
                "machine-semantic-container-manifest.schema.json"
            ],
            semantic_manifest,
        )
        if semantic_manifest_errors:
            raise ValidationFailure(
                "private 1.4 semantic-container manifest was rejected: "
                + " | ".join(semantic_manifest_errors)
            )
        for path, value, label in (
            (semantic_document_path, semantic_document, "DocumentPackage"),
            (semantic_manifest_path, semantic_manifest, "semantic manifest"),
        ):
            if path.read_bytes().rstrip(b"\n") != jcs_bytes(value):
                raise ValidationFailure(
                    f"private 1.4 {label} fixture is not canonical JCS"
                )

        semantic_job = STAGING_SEMANTIC_CONTAINER_FIXTURE_DIR / "job"
        for source in semantic_document["sources"]:
            source_bytes = (semantic_job / source["uri"]).read_bytes()
            if (
                len(source_bytes) != source["utf8_byte_length"]
                or hashlib.sha256(source_bytes).hexdigest() != source["sha256"]
            ):
                raise ValidationFailure("private 1.4 source attestation drifted")
        declarations = [
            *(('font', item) for item in semantic_document["resources"]["font_faces"]),
            *(('image', item) for item in semantic_document["resources"]["images"]),
        ]
        if len(declarations) != len(semantic_manifest["resources"]):
            raise ValidationFailure("private 1.4 declared-media coverage is incomplete")
        for (resource_kind, declaration), record in zip(
            declarations, semantic_manifest["resources"], strict=True
        ):
            resource_id_name = "font_face_id" if resource_kind == "font" else "image_id"
            resource_bytes = (semantic_job / declaration["uri"]).read_bytes()
            if (
                record["resource_kind"] != resource_kind
                or record["resource_id"] != declaration[resource_id_name]
                or record["media_declaration"]
                != {"kind": "declared", "media_type": declaration["media_type"]}
                or record["attested_media_kind"] != declaration["media_type"]
                or record["sha256"] != hashlib.sha256(resource_bytes).hexdigest()
                or record["sha256"] != declaration["expected_sha256"]
            ):
                raise ValidationFailure(
                    "private 1.4 declaration/attestation closure drifted"
                )

        semantic_containers: dict[int, tuple[str, dict[str, Any], list[int]]] = {}

        def collect_semantic_containers(blocks: list[dict[str, Any]]) -> None:
            for block in blocks:
                kind = block["kind"]
                if kind == "semantic_container":
                    semantic_containers[block["node_id"]] = (
                        block["semantic_kind"],
                        block["span"],
                        [child["node_id"] for child in block["blocks"]],
                    )
                    collect_semantic_containers(block["blocks"])
                elif kind == "list":
                    for item in block["items"]:
                        collect_semantic_containers(item["blocks"])
                elif kind == "table":
                    for row in [*block["head"], *block["body"]]:
                        for cell in row["cells"]:
                            collect_semantic_containers(cell["blocks"])
                elif kind == "figure":
                    collect_semantic_containers(block["caption"])

        collect_semantic_containers(semantic_document["document"]["blocks"])
        for footnote in semantic_document["document"]["footnotes"]:
            collect_semantic_containers(footnote["blocks"])
        observed_fragments: dict[int, list[dict[str, Any]]] = {}
        for expected_page, fact in enumerate(semantic_manifest["selected_facts"]):
            observed_fragments.setdefault(fact["owner"], []).append(fact)
            container = semantic_containers.get(fact["owner"])
            if (
                container is None
                or fact["kind"] != container[0]
                or fact["source_span"] != container[1]
                or fact["page_index"] != expected_page
            ):
                raise ValidationFailure(
                    "private 1.4 selected semantic owner/kind/span/page drifted"
                )
        if set(observed_fragments) != set(semantic_containers):
            raise ValidationFailure(
                "private 1.4 selected facts do not cover every container"
            )
        for owner, facts in observed_fragments.items():
            if [fact["fragment_index"] for fact in facts] != list(range(len(facts))):
                raise ValidationFailure(
                    "private 1.4 semantic fragment indices are not dense"
                )
            if [child for fact in facts for child in fact["child_owners"]] != (
                semantic_containers[owner][2]
            ):
                raise ValidationFailure(
                    "private 1.4 semantic fragment child closure drifted"
                )

        unknown_semantic_kind = copy.deepcopy(semantic_document)
        unknown_semantic_kind["document"]["blocks"][0]["semantic_kind"] = "lemma"
        empty_semantic = copy.deepcopy(semantic_document)
        empty_semantic["document"]["blocks"][0]["blocks"] = []
        missing_media = copy.deepcopy(semantic_document)
        del missing_media["resources"]["images"][0]["media_type"]
        inline_semantic = copy.deepcopy(semantic_document)
        inline_semantic["document"]["blocks"][0]["blocks"][0]["children"].append(
            copy.deepcopy(semantic_document["document"]["blocks"][0])
        )
        for label, invalid in (
            ("unknown kind", unknown_semantic_kind),
            ("empty blocks", empty_semantic),
            ("missing media", missing_media),
            ("inline container", inline_semantic),
        ):
            if not schema_errors(
                private_m4_validators["document-package.schema.json"], invalid
            ):
                raise ValidationFailure(
                    f"private 1.4 DocumentPackage accepted {label}"
                )
        mismatched_media = copy.deepcopy(semantic_manifest)
        mismatched_media["resources"][0]["attested_media_kind"] = (
            "ttc-truetype-glyf"
        )
        if not schema_errors(
            private_m4_validators[
                "machine-semantic-container-manifest.schema.json"
            ],
            mismatched_media,
        ):
            raise ValidationFailure(
                "private 1.4 manifest accepted declared/attested media mismatch"
            )

        vector_document_path = STAGING_SAFE_VECTOR_FIXTURE_DIR / "job" / "document-package.json"
        vector_manifest_path = STAGING_SAFE_VECTOR_FIXTURE_DIR / "manifest.json"
        vector_document = load_json(vector_document_path)
        vector_manifest = load_json(vector_manifest_path)
        vector_document_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], vector_document
        )
        if vector_document_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the SafeVector fixture: "
                + " | ".join(vector_document_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            vector_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private SafeVector fixture"
            )
        vector_manifest_errors = schema_errors(
            private_m4_validators["machine-safe-vector-manifest.schema.json"],
            vector_manifest,
        )
        if vector_manifest_errors:
            raise ValidationFailure(
                "private 1.4 SafeVector manifest was rejected: "
                + " | ".join(vector_manifest_errors)
            )
        for path, value, label in (
            (vector_document_path, vector_document, "SafeVector DocumentPackage"),
            (vector_manifest_path, vector_manifest, "SafeVector manifest"),
        ):
            if path.read_bytes().rstrip(b"\n") != jcs_bytes(value):
                raise ValidationFailure(f"private 1.4 {label} is not canonical JCS")
        vector_declarations = vector_document["resources"]["images"]
        vector_resources = vector_manifest["resources"]
        if len(vector_declarations) != len(vector_resources):
            raise ValidationFailure("private 1.4 SafeVector resource coverage is incomplete")
        vector_job = STAGING_SAFE_VECTOR_FIXTURE_DIR / "job"
        for declaration, record in zip(vector_declarations, vector_resources, strict=True):
            resource_bytes = (vector_job / declaration["uri"]).read_bytes()
            if (
                declaration["media_type"] != "svg-safe-1"
                or record["image_id"] != declaration["image_id"]
                or record["uri"] != declaration["uri"]
                or record["declared_media_type"] != declaration["media_type"]
                or record["attested_media_kind"] != declaration["media_type"]
                or record["admitted_sha256"] != hashlib.sha256(resource_bytes).hexdigest()
                or record["admitted_sha256"] != declaration["expected_sha256"]
                or bool(record["usages"]) != (record["form_plan_fingerprint"] is not None)
                or bool(record["usages"]) != (record["pdf_form_object_number"] is not None)
                or bool(record["usages"]) != (record["pdf_resource_name"] is not None)
            ):
                raise ValidationFailure("private 1.4 SafeVector closure drifted")
        wrong_vector_media = copy.deepcopy(vector_document)
        wrong_vector_media["resources"]["images"][0]["media_type"] = "image/svg+xml"
        if not schema_errors(
            private_m4_validators["document-package.schema.json"], wrong_vector_media
        ):
            raise ValidationFailure("private 1.4 accepted unknown SafeVector media")
        mismatched_vector_manifest = copy.deepcopy(vector_manifest)
        mismatched_vector_manifest["resources"][0]["attested_media_kind"] = "png"
        if not schema_errors(
            private_m4_validators["machine-safe-vector-manifest.schema.json"],
            mismatched_vector_manifest,
        ):
            raise ValidationFailure("private 1.4 SafeVector manifest accepted media mismatch")

        precomposed_vector_document_path = (
            STAGING_PRECOMPOSED_VECTOR_FIXTURE_DIR / "document-package.json"
        )
        precomposed_vector_document = load_json(precomposed_vector_document_path)
        precomposed_vector_errors = schema_errors(
            private_m4_validators["document-package.schema.json"],
            precomposed_vector_document,
        )
        if precomposed_vector_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the precomposed-vector fixture: "
                + " | ".join(precomposed_vector_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            precomposed_vector_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private precomposed-vector fixture"
            )
        if (
            precomposed_vector_document_path.read_bytes().rstrip(b"\n")
            != jcs_bytes(precomposed_vector_document)
        ):
            raise ValidationFailure(
                "private 1.4 precomposed-vector DocumentPackage is not canonical JCS"
            )

        precomposed_vector_source = (
            STAGING_PRECOMPOSED_VECTOR_FIXTURE_DIR / "input.tsf"
        ).read_bytes()
        precomposed_vector_declaration = precomposed_vector_document["resources"][
            "images"
        ][0]
        precomposed_vector_resource = (
            STAGING_PRECOMPOSED_VECTOR_FIXTURE_DIR
            / precomposed_vector_declaration["uri"]
        ).read_bytes()
        if (
            precomposed_vector_document["sources"]
            != [
                {
                    "sha256": hashlib.sha256(precomposed_vector_source).hexdigest(),
                    "source_id": 0,
                    "uri": "input.tsf",
                    "utf8_byte_length": len(precomposed_vector_source),
                }
            ]
            or precomposed_vector_declaration["media_type"] != "svg-safe-2"
            or precomposed_vector_declaration["expected_sha256"]
            != hashlib.sha256(precomposed_vector_resource).hexdigest()
            or set(precomposed_vector_declaration["vector_provenance"])
            != {"engine_id", "engine_version", "rules_version"}
        ):
            raise ValidationFailure(
                "private 1.4 precomposed-vector source/resource closure drifted"
            )

        precomposed_vector_blocks = precomposed_vector_document["document"]["blocks"][
            0
        ]["blocks"]
        precomposed_vector_inlines = precomposed_vector_blocks[0]["children"]
        if {
            *(inline["kind"] for inline in precomposed_vector_inlines),
            *(block["kind"] for block in precomposed_vector_blocks[1:]),
        } != {
            "inline_vector",
            "math_vector",
            "vector_figure",
            "math_vector_block",
        }:
            raise ValidationFailure(
                "private 1.4 precomposed-vector fixture does not cover all four kinds"
            )

        inline_vector_trace_path = (
            STAGING_PRECOMPOSED_VECTOR_FIXTURE_DIR / "inline-layout-trace.json"
        )
        inline_vector_trace = load_json(inline_vector_trace_path)
        inline_vector_trace_errors = schema_errors(
            private_m4_validators["layout-trace.schema.json"], inline_vector_trace
        )
        if inline_vector_trace_errors:
            raise ValidationFailure(
                "private 1.4 inline-vector layout trace was rejected: "
                + " | ".join(inline_vector_trace_errors)
            )
        if (
            inline_vector_trace_path.read_bytes().rstrip(b"\n")
            != jcs_bytes(inline_vector_trace)
        ):
            raise ValidationFailure(
                "private 1.4 inline-vector layout trace is not canonical JCS"
            )
        inline_layout = inline_vector_trace["precomposed_vector_layout"]
        inline_lines = inline_layout["lines"]
        inline_placements = inline_layout["placements"]
        if (
            inline_layout["line_count"] != len(inline_lines)
            or inline_layout["placement_count"] != len(inline_placements)
            or inline_layout["fragment_charge"]
            != len(inline_lines) + len(inline_placements)
            or [line["record"]["line_index"] for line in inline_lines]
            != list(range(len(inline_lines)))
            or [placement["record"]["occurrence"] for placement in inline_placements]
            != list(range(len(inline_placements)))
            or [placement["record"]["paint_ordinal"] for placement in inline_placements]
            != list(range(len(inline_placements)))
        ):
            raise ValidationFailure(
                "private 1.4 inline-vector layout counts or dense ordinals drifted"
            )
        for placement_wrapper in inline_placements:
            placement = placement_wrapper["record"]
            line = inline_lines[placement["line_index"]]["record"]
            viewport = placement["viewport"]
            if (
                viewport["y"] + placement["baseline"]
                != placement["baseline_y"]
                or placement["baseline_y"] != line["baseline_y"]
                or placement["page_index"] != line["page_index"]
                or placement["frame_index"] != line["frame_index"]
                or placement["fragment_ordinal"] != line["fragment_ordinal"]
                or viewport["y"] < line["line_top"]
                or viewport["y"] + viewport["height"]
                > line["line_top"] + line["line_height"]
            ):
                raise ValidationFailure(
                    "private 1.4 inline-vector baseline or line containment drifted"
                )

        block_vector_trace_path = (
            STAGING_PRECOMPOSED_VECTOR_FIXTURE_DIR / "block-layout-trace.json"
        )
        block_vector_trace = load_json(block_vector_trace_path)
        block_vector_trace_errors = schema_errors(
            private_m4_validators["layout-trace.schema.json"], block_vector_trace
        )
        if block_vector_trace_errors:
            raise ValidationFailure(
                "private 1.4 block-vector layout trace was rejected: "
                + " | ".join(block_vector_trace_errors)
            )
        if (
            block_vector_trace_path.read_bytes().rstrip(b"\n")
            != jcs_bytes(block_vector_trace)
        ):
            raise ValidationFailure(
                "private 1.4 block-vector layout trace is not canonical JCS"
            )

        block_layout = block_vector_trace["precomposed_vector_block_layout"]
        block_placements = block_layout["block_placements"]
        block_pages = block_layout["pages"]
        fragment_charge = block_layout["fragment_charge"]
        prior_fragment_charge = block_layout["pagination_input"][
            "prior_fragment_charge"
        ]
        referenced_block_pages = [
            placement["record"]["page_index"] for placement in block_placements
        ]
        referenced_caption_pages = [
            caption["page_index"]
            for placement in block_placements
            for caption in placement["record"]["captions"]
        ]
        if (
            block_layout["block_placement_count"] != len(block_placements)
            or fragment_charge != len(block_placements)
            or block_layout["pagination_input_fingerprint"]
            != hashlib.sha256(
                jcs_bytes(block_layout["pagination_input"])
            ).hexdigest()
            or block_layout["pagination_input"]["preparation_fingerprint"]
            != block_layout["preparation_fingerprint"]
            or block_layout["page_geometry_fingerprint"]
            != hashlib.sha256(jcs_bytes(block_layout["page_geometry"])).hexdigest()
            or block_layout["cumulative_fragment_charge"]
            != prior_fragment_charge + fragment_charge
            or [placement["record"]["block_ordinal"] for placement in block_placements]
            != list(range(len(block_placements)))
            or [
                placement["record"]["fragment_ordinal"]
                for placement in block_placements
            ]
            != list(range(len(block_placements)))
            or [page["page_index"] for page in block_pages]
            != list(range(len(block_pages)))
            or any(
                page_index >= len(block_pages)
                for page_index in referenced_block_pages + referenced_caption_pages
            )
        ):
            raise ValidationFailure(
                "private 1.4 block-vector layout counts or dense ordinals drifted"
            )

        block_body = block_layout["page_geometry"]["body"]

        def block_rect_contains(container: dict[str, int], child: dict[str, int]) -> bool:
            return (
                child["x"] >= container["x"]
                and child["y"] >= container["y"]
                and child["x"] + child["width"]
                <= container["x"] + container["width"]
                and child["y"] + child["height"]
                <= container["y"] + container["height"]
            )

        paint_ordinals: list[int] = []
        for placement_wrapper in block_placements:
            placement = placement_wrapper["record"]
            if placement_wrapper["fingerprint"] != hashlib.sha256(
                jcs_bytes(placement)
            ).hexdigest():
                raise ValidationFailure(
                    "private 1.4 block-vector placement fingerprint drifted"
                )

            pagination_bounds = placement["pagination_bounds"]
            paint_bounds = placement["paint_bounds"]
            structure_bounds = placement["structure_bounds"]
            viewport = placement["viewport"]
            viewport_rect = viewport["rect"]
            matrix = viewport["matrix"]
            children = placement["structure_children"]
            if (
                pagination_bounds != paint_bounds
                or pagination_bounds != structure_bounds
                or matrix["a_16_16"] != viewport["scale"]
                or matrix["d_16_16"] != viewport["scale"]
                or matrix["b_16_16"] != 0
                or matrix["c_16_16"] != 0
                or matrix["e"] != viewport_rect["x"]
                or matrix["f"] != viewport_rect["y"]
                or not block_rect_contains(block_body, pagination_bounds)
                or not block_rect_contains(pagination_bounds, viewport_rect)
            ):
                raise ValidationFailure(
                    "private 1.4 block-vector geometry or viewport matrix drifted"
                )

            if (
                not children
                or children[0]["rect"] != viewport_rect
                or children[0]["page_index"] != placement["page_index"]
                or children[0]["paint_ordinal"] != viewport["paint_ordinal"]
            ):
                raise ValidationFailure(
                    "private 1.4 block-vector primary structure child drifted"
                )
            paint_ordinals.extend(child["paint_ordinal"] for child in children)

            equation_number = placement["equation_number"]
            if placement["kind"] == "math_vector_block":
                baseline = placement["math_baseline"]
                math_flow = placement["math_flow"]
                if (
                    baseline is None
                    or math_flow is None
                    or viewport_rect["y"] + baseline["baseline"]
                    != baseline["baseline_y"]
                    or children[0]["role"] != "formula"
                    or children[0]["owner"] != placement["node_id"]
                    or placement["captions"]
                    or placement["keep_caption"]
                ):
                    raise ValidationFailure(
                        "private 1.4 block-vector math baseline or structure drifted"
                    )
                if equation_number is None:
                    if len(children) != 1:
                        raise ValidationFailure(
                            "private 1.4 unnumbered math-vector children drifted"
                        )
                elif (
                    len(children) != 2
                    or children[1]["role"] != "equation_number"
                    or children[1]["owner"] != equation_number["owner"]
                    or children[1]["rect"] != equation_number["rect"]
                    or children[1]["paint_ordinal"]
                    != equation_number["paint_ordinal"]
                    or not block_rect_contains(
                        pagination_bounds, equation_number["rect"]
                    )
                    or equation_number["rect"]["x"]
                    < viewport_rect["x"]
                    + viewport_rect["width"]
                    + equation_number["minimum_gap"]
                ):
                    raise ValidationFailure(
                        "private 1.4 block-vector equation-number placement drifted"
                    )
            elif (
                placement["kind"] != "vector_figure"
                or placement["math_baseline"] is not None
                or placement["math_flow"] is not None
                or equation_number is not None
                or children[0]["role"] != "figure"
                or children[0]["owner"] != placement["node_id"]
            ):
                raise ValidationFailure(
                    "private 1.4 block-vector Figure structure drifted"
                )

            caption_children = [
                child for child in children if child["role"] == "caption"
            ]
            if len(caption_children) != len(placement["captions"]) or any(
                child["owner"] != caption["owner"]
                or child["page_index"] != caption["page_index"]
                or child["paint_ordinal"] != caption["paint_ordinal"]
                or child["rect"] != caption["rect"]
                or not block_rect_contains(block_body, caption["rect"])
                for child, caption in zip(caption_children, placement["captions"])
            ):
                raise ValidationFailure(
                    "private 1.4 block-vector caption structure drifted"
                )

        if paint_ordinals != list(range(len(paint_ordinals))):
            raise ValidationFailure(
                "private 1.4 block-vector paint ordinals are not dense"
            )

        for page in block_pages:
            page_placements = [
                placement["record"]
                for placement in block_placements
                if placement["record"]["page_index"] == page["page_index"]
            ]
            if (
                page["block_count"] != len(page_placements)
                or page["caption_count"]
                != sum(
                    1
                    for placement in block_placements
                    for caption in placement["record"]["captions"]
                    if caption["page_index"] == page["page_index"]
                )
                or [placement["page_block_ordinal"] for placement in page_placements]
                != list(range(len(page_placements)))
            ):
                raise ValidationFailure(
                    "private 1.4 block-vector page accounting drifted"
                )

        mixed_layout_trace = copy.deepcopy(block_vector_trace)
        mixed_layout_trace["precomposed_vector_layout"] = inline_layout
        if not schema_errors(
            private_m4_validators["layout-trace.schema.json"], mixed_layout_trace
        ):
            raise ValidationFailure(
                "private 1.4 layout trace accepted mixed inline/block evidence"
            )
        wrong_block_child_role = copy.deepcopy(block_vector_trace)
        wrong_block_child_role["precomposed_vector_block_layout"]["block_placements"][
            0
        ]["record"]["structure_children"][0]["role"] = "formula"
        unnumbered_with_number_child = copy.deepcopy(block_vector_trace)
        unnumbered_math = unnumbered_with_number_child[
            "precomposed_vector_block_layout"
        ]["block_placements"][1]["record"]
        unnumbered_math["equation_number"] = None
        for label, invalid_trace in (
            ("wrong Figure child role", wrong_block_child_role),
            ("number child on unnumbered math", unnumbered_with_number_child),
        ):
            if not schema_errors(
                private_m4_validators["layout-trace.schema.json"], invalid_trace
            ):
                raise ValidationFailure(
                    f"private 1.4 layout trace accepted {label}"
                )

        def require_invalid_precomposed_vector(
            label: str, invalid: dict[str, Any]
        ) -> None:
            if not schema_errors(
                private_m4_validators["document-package.schema.json"], invalid
            ):
                raise ValidationFailure(
                    f"private 1.4 DocumentPackage accepted invalid precomposed-vector {label}"
                )

        missing_safe2_hash = copy.deepcopy(precomposed_vector_document)
        del missing_safe2_hash["resources"]["images"][0]["expected_sha256"]
        null_safe2_hash = copy.deepcopy(precomposed_vector_document)
        null_safe2_hash["resources"]["images"][0]["expected_sha256"] = None
        missing_provenance = copy.deepcopy(precomposed_vector_document)
        del missing_provenance["resources"]["images"][0]["vector_provenance"]
        invalid_provenance = copy.deepcopy(precomposed_vector_document)
        invalid_provenance["resources"]["images"][0]["vector_provenance"][
            "engine_id"
        ] = "vmb\ntexToSvg"
        provenance_on_safe1 = copy.deepcopy(precomposed_vector_document)
        provenance_on_safe1["resources"]["images"][0]["media_type"] = "svg-safe-1"
        missing_actual_text = copy.deepcopy(precomposed_vector_document)
        del missing_actual_text["document"]["blocks"][0]["blocks"][0]["children"][
            0
        ]["actual_text"]
        source_tex_on_inline_vector = copy.deepcopy(precomposed_vector_document)
        source_tex_on_inline_vector["document"]["blocks"][0]["blocks"][0][
            "children"
        ][0]["source_tex"] = copy.deepcopy(
            precomposed_vector_inlines[1]["source_tex"]
        )
        missing_math_source_tex = copy.deepcopy(precomposed_vector_document)
        del missing_math_source_tex["document"]["blocks"][0]["blocks"][0][
            "children"
        ][1]["source_tex"]
        wrong_source_text_id_type = copy.deepcopy(precomposed_vector_document)
        wrong_source_text_id_type["document"]["blocks"][0]["blocks"][0][
            "children"
        ][1]["source_tex"]["text_span"]["text_id"] = "0"
        missing_equation_number = copy.deepcopy(precomposed_vector_document)
        del missing_equation_number["document"]["blocks"][0]["blocks"][2][
            "equation_number"
        ]
        zero_advance = copy.deepcopy(precomposed_vector_document)
        zero_advance["document"]["blocks"][0]["blocks"][0]["children"][0][
            "metrics"
        ]["advance"] = 0
        wrong_viewport_width_type = copy.deepcopy(precomposed_vector_document)
        wrong_viewport_width_type["document"]["blocks"][0]["blocks"][1][
            "viewport"
        ]["width"] = "1966080"
        unknown_precomposed_vector_kind = copy.deepcopy(precomposed_vector_document)
        unknown_precomposed_vector_kind["document"]["blocks"][0]["blocks"][0][
            "children"
        ][0]["kind"] = "vector"
        for label, invalid in (
            ("missing svg-safe-2 hash", missing_safe2_hash),
            ("null svg-safe-2 hash", null_safe2_hash),
            ("missing provenance", missing_provenance),
            ("invalid provenance", invalid_provenance),
            ("provenance on svg-safe-1", provenance_on_safe1),
            ("missing actual_text", missing_actual_text),
            ("source_tex on inline_vector", source_tex_on_inline_vector),
            ("missing math source_tex", missing_math_source_tex),
            ("wrong source text_id type", wrong_source_text_id_type),
            ("missing equation_number", missing_equation_number),
            ("zero advance", zero_advance),
            ("wrong viewport width type", wrong_viewport_width_type),
            ("unknown kind", unknown_precomposed_vector_kind),
        ):
            require_invalid_precomposed_vector(label, invalid)

        math_document_path = STAGING_MATH_FIXTURE_DIR / "job" / "document-package.json"
        math_page_document_path = (
            STAGING_MATH_FIXTURE_DIR / "job" / "page-document-package.json"
        )
        math_keep_document_path = (
            STAGING_MATH_FIXTURE_DIR / "job" / "keep-document-package.json"
        )
        math_manifest_path = STAGING_MATH_FIXTURE_DIR / "manifest.json"
        math_document = load_json(math_document_path)
        math_page_document = load_json(math_page_document_path)
        math_keep_document = load_json(math_keep_document_path)
        math_manifest = load_json(math_manifest_path)
        math_document_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], math_document
        )
        if math_document_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the math fixture: "
                + " | ".join(math_document_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            math_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private math fixture"
            )
        math_page_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], math_page_document
        )
        if math_page_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the math page fixture: "
                + " | ".join(math_page_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            math_page_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private math page fixture"
            )
        math_keep_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], math_keep_document
        )
        if math_keep_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the math keep fixture: "
                + " | ".join(math_keep_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            math_keep_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private math keep fixture"
            )
        math_manifest_errors = schema_errors(
            private_m4_validators["machine-math-manifest.schema.json"],
            math_manifest,
        )
        if math_manifest_errors:
            raise ValidationFailure(
                "private 1.4 math manifest was rejected: "
                + " | ".join(math_manifest_errors)
            )
        for path, value, label in (
            (math_document_path, math_document, "math DocumentPackage"),
            (math_page_document_path, math_page_document, "math page DocumentPackage"),
            (math_keep_document_path, math_keep_document, "math keep DocumentPackage"),
            (math_manifest_path, math_manifest, "math manifest"),
        ):
            if path.read_bytes().rstrip(b"\n") != jcs_bytes(value):
                raise ValidationFailure(f"private 1.4 {label} is not canonical JCS")
        math_font = (STAGING_MATH_FIXTURE_DIR / "job" / "math.ttf").read_bytes()
        for fixture, source_name, label in (
            (math_document, "input.tsf", "math"),
            (math_page_document, "input.tsf", "math page"),
            (math_keep_document, "keep-input.tsf", "math keep"),
        ):
            source_bytes = (
                STAGING_MATH_FIXTURE_DIR / "job" / source_name
            ).read_bytes()
            if (
                len(fixture["sources"]) != 1
                or len(fixture["text_buffers"]) != 1
                or len(fixture["resources"]["font_faces"]) != 1
            ):
                raise ValidationFailure(
                    f"private 1.4 {label} fixture catalog closure drifted"
                )
            source_declaration = fixture["sources"][0]
            font_declaration = fixture["resources"]["font_faces"][0]
            if (
                source_declaration["source_id"] != 0
                or source_declaration["uri"] != source_name
                or source_declaration["utf8_byte_length"] != len(source_bytes)
                or source_declaration["sha256"]
                != hashlib.sha256(source_bytes).hexdigest()
                or fixture["text_buffers"][0]["utf8"].encode("utf-8")
                != source_bytes
                or font_declaration["font_face_id"] != 0
                or font_declaration["face_index"] != 0
                or font_declaration["uri"] != "math.ttf"
                or font_declaration["media_type"] != "sfnt-truetype-glyf"
                or font_declaration["expected_sha256"]
                != hashlib.sha256(math_font).hexdigest()
            ):
                raise ValidationFailure(
                    f"private 1.4 {label} source/font declaration drifted"
                )
        math_source = (STAGING_MATH_FIXTURE_DIR / "job" / "input.tsf").read_bytes()
        math_nodes = [
            math_document["document"]["blocks"][0]["blocks"][0]["children"][0],
            math_document["document"]["blocks"][0]["blocks"][1],
        ]
        if len(math_nodes) != len(math_manifest["facts"]):
            raise ValidationFailure("private 1.4 math occurrence coverage is incomplete")
        for occurrence, (node, fact) in enumerate(
            zip(math_nodes, math_manifest["facts"], strict=True)
        ):
            source_span = node["span"]
            text_span = node["math_source"]["text_span"]
            selected = math_source[text_span["start_byte"] : text_span["end_byte"]]
            if (
                fact["occurrence"] != occurrence
                or fact["node_id"] != node["node_id"]
                or fact["kind"] != node["kind"]
                or fact["source"]["language"] != node["math_source"]["language"]
                or fact["source"]["version"] != node["math_source"]["version"]
                or fact["source"]["source_span"] != source_span
                or fact["source"]["text_span"] != text_span
                or fact["source"]["sha256"] != hashlib.sha256(selected).hexdigest()
                or fact["speech_sha256"]
                != hashlib.sha256(node["speech"].encode("utf-8")).hexdigest()
                or fact["actual_text_sha256"] != fact["speech_sha256"]
                or fact["selected"]["page_index"] < 0
                or fact["pdf"]["page_object"] <= 0
            ):
                raise ValidationFailure("private 1.4 math receipt closure drifted")
        wrong_math_version = copy.deepcopy(math_document)
        wrong_math_version["document"]["blocks"][0]["blocks"][1]["math_source"][
            "version"
        ] = "2"
        if not schema_errors(
            private_m4_validators["document-package.schema.json"], wrong_math_version
        ):
            raise ValidationFailure("private 1.4 accepted an unknown math source version")
        whitespace_math_speech = copy.deepcopy(math_document)
        whitespace_math_speech["document"]["blocks"][0]["blocks"][0][
            "children"
        ][0]["speech"] = "\u2007"
        if not schema_errors(
            private_m4_validators["document-package.schema.json"],
            whitespace_math_speech,
        ):
            raise ValidationFailure("private 1.4 accepted whitespace-only math speech")
        page_region_math = copy.deepcopy(math_document)
        page_region_master = page_region_math["page_masters"]["masters"][0]
        page_region_master["header"] = copy.deepcopy(page_region_master["body"])
        page_region_paragraph = copy.deepcopy(
            math_document["document"]["blocks"][0]["blocks"][0]
        )
        page_region_paragraph["children"] = [
            {
                "kind": "text",
                "node_id": 3,
                "span": {"end_byte": 5, "source_id": 0, "start_byte": 0},
                "text_span": {"end_byte": 5, "start_byte": 0, "text_id": 0},
            }
        ]
        page_region_master["header_content"] = {
            "blocks": [page_region_paragraph],
            "node_id": 1,
            "span": {"end_byte": 5, "source_id": 0, "start_byte": 0},
        }
        page_region_text_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], page_region_math
        )
        if page_region_text_errors:
            raise ValidationFailure(
                "private 1.4 rejected the valid restricted page-region control: "
                + " | ".join(page_region_text_errors)
            )
        page_region_paragraph["children"] = [
            copy.deepcopy(
                math_document["document"]["blocks"][0]["blocks"][0]["children"][0]
            )
        ]
        if not schema_errors(
            private_m4_validators["document-package.schema.json"], page_region_math
        ):
            raise ValidationFailure("private 1.4 accepted math in a page region")
        wrong_math_parser = copy.deepcopy(math_manifest)
        wrong_math_parser["facts"][0]["parser"] = "host.math-parser/1"
        if not schema_errors(
            private_m4_validators["machine-math-manifest.schema.json"],
            wrong_math_parser,
        ):
            raise ValidationFailure("private 1.4 math manifest accepted a foreign parser")

        book_document_path = (
            STAGING_BOOK_NAVIGATION_FIXTURE_DIR / "job" / "document-package.json"
        )
        book_manifest_path = STAGING_BOOK_NAVIGATION_FIXTURE_DIR / "manifest.json"
        book_expectation_path = (
            STAGING_BOOK_NAVIGATION_FIXTURE_DIR / "pdf-expectation.json"
        )
        book_document = load_json(book_document_path)
        book_manifest = load_json(book_manifest_path)
        book_expectation = load_json(book_expectation_path)
        book_document_errors = schema_errors(
            private_m4_validators["document-package.schema.json"], book_document
        )
        if book_document_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the book-navigation fixture: "
                + " | ".join(book_document_errors)
            )
        if not schema_errors(
            versioned_current_validators["document-package.schema.json"],
            book_document,
        ):
            raise ValidationFailure(
                "versioned 1.3 DocumentPackage accepted the private book-navigation fixture"
            )
        book_manifest_errors = schema_errors(
            private_m4_validators[
                "machine-book-navigation-manifest.schema.json"
            ],
            book_manifest,
        )
        if book_manifest_errors:
            raise ValidationFailure(
                "private 1.4 book-navigation manifest was rejected: "
                + " | ".join(book_manifest_errors)
            )
        for path, value, label in (
            (book_document_path, book_document, "book-navigation DocumentPackage"),
            (book_manifest_path, book_manifest, "book-navigation manifest"),
            (book_expectation_path, book_expectation, "book-navigation PDF expectation"),
        ):
            if path.read_bytes().rstrip(b"\n") != jcs_bytes(value):
                raise ValidationFailure(f"private 1.4 {label} is not canonical JCS")
        validate_book_navigation_semantics(book_document, book_manifest)
        if canonical_book_language("SGN-be-fr") != "sgn-BE-FR":
            raise ValidationFailure(
                "book-navigation grandfathered language casing drifted"
            )

        source_bytes = (
            STAGING_BOOK_NAVIGATION_FIXTURE_DIR / "job" / "input.tsf"
        ).read_bytes()
        source = book_document["sources"][0]
        if (
            len(book_document["sources"]) != 1
            or source["uri"] != "input.tsf"
            or source["utf8_byte_length"] != len(source_bytes)
            or source["sha256"] != hashlib.sha256(source_bytes).hexdigest()
            or book_document["text_buffers"][0]["utf8"].encode("utf-8")
            != source_bytes
        ):
            raise ValidationFailure("private 1.4 book-navigation source closure drifted")
        if (
            book_expectation["metadata"] != book_document["metadata"]
            or book_expectation["document_language"]
            != canonical_book_language(book_document["document"]["language"])
            or len(book_expectation["outline"])
            != len(book_document["outline"]["entries"])
        ):
            raise ValidationFailure("private 1.4 PDF expectation differs from source facts")

        missing_book_metadata = copy.deepcopy(book_document)
        del missing_book_metadata["metadata"]
        null_book_language = copy.deepcopy(book_document)
        null_book_language["document"]["blocks"][0]["blocks"][0][
            "language"
        ] = None
        for label, invalid in (
            ("missing metadata", missing_book_metadata),
            ("null node language", null_book_language),
        ):
            if not schema_errors(
                private_m4_validators["document-package.schema.json"], invalid
            ):
                raise ValidationFailure(
                    f"private 1.4 DocumentPackage accepted book-navigation {label}"
                )

        bad_book_date = copy.deepcopy(book_document)
        bad_book_date["metadata"]["created"] = "2026-02-30T00:00:00Z"
        unordered_book_keywords = copy.deepcopy(book_document)
        unordered_book_keywords["metadata"]["keywords"] = [
            "typesetting",
            "determinism",
        ]
        bad_book_language = copy.deepcopy(book_document)
        bad_book_language["document"]["blocks"][0]["blocks"][0][
            "language"
        ] = "fr_Latn_FR"
        bad_book_parent = copy.deepcopy(book_document)
        bad_book_parent["outline"]["entries"][2]["parent_outline_id"] = None
        duplicate_book_destination = copy.deepcopy(book_document)
        duplicate_book_destination["outline"]["entries"][2][
            "destination"
        ] = "chapter-1"
        duplicate_book_anchor = copy.deepcopy(book_document)
        duplicate_book_anchor["document"]["blocks"][0]["blocks"][2][
            "anchor_id"
        ] = "chapter-1"
        for label, invalid in (
            ("non-Gregorian date", bad_book_date),
            ("unordered keywords", unordered_book_keywords),
            ("malformed language", bad_book_language),
            ("wrong outline parent", bad_book_parent),
            ("duplicate outline destination", duplicate_book_destination),
            ("duplicate package anchor", duplicate_book_anchor),
        ):
            try:
                validate_book_navigation_semantics(invalid)
            except ValidationFailure:
                pass
            else:
                raise ValidationFailure(
                    f"private 1.4 semantic validation accepted {label}"
                )
        mismatched_book_manifest = copy.deepcopy(book_manifest)
        mismatched_book_manifest["metadata"]["title"] = "Foreign title"
        try:
            validate_book_navigation_semantics(
                book_document, mismatched_book_manifest
            )
        except ValidationFailure:
            pass
        else:
            raise ValidationFailure(
                "private 1.4 manifest accepted mismatched source metadata"
            )
        zero_outline_root = copy.deepcopy(book_manifest)
        zero_outline_root["pdf"]["outline_root_object"] = 0
        if not schema_errors(
            private_m4_validators[
                "machine-book-navigation-manifest.schema.json"
            ],
            zero_outline_root,
        ):
            raise ValidationFailure(
                "private 1.4 book-navigation manifest accepted object zero"
            )

        accessibility_document_path = (
            STAGING_ACCESSIBILITY_FIXTURE_DIR / "job" / "document-package.json"
        )
        accessibility_manifest_path = STAGING_ACCESSIBILITY_FIXTURE_DIR / "manifest.json"
        accessibility_pdf_path = STAGING_ACCESSIBILITY_FIXTURE_DIR / "output.pdf"
        accessibility_document = load_json(accessibility_document_path)
        accessibility_manifest = load_json(accessibility_manifest_path)
        accessibility_document_errors = schema_errors(
            private_m4_validators["document-package.schema.json"],
            accessibility_document,
        )
        if accessibility_document_errors:
            raise ValidationFailure(
                "private 1.4 DocumentPackage rejected the accessibility fixture: "
                + " | ".join(accessibility_document_errors)
            )
        accessibility_manifest_errors = schema_errors(
            private_m4_validators[
                "machine-accessibility-manifest.schema.json"
            ],
            accessibility_manifest,
        )
        if accessibility_manifest_errors:
            raise ValidationFailure(
                "private 1.4 accessibility manifest was rejected: "
                + " | ".join(accessibility_manifest_errors)
            )
        for path, value, label in (
            (accessibility_document_path, accessibility_document, "accessibility DocumentPackage"),
            (accessibility_manifest_path, accessibility_manifest, "accessibility manifest"),
        ):
            if path.read_bytes().rstrip(b"\n") != jcs_bytes(value):
                raise ValidationFailure(f"private 1.4 {label} is not canonical JCS")
        accessibility_pdf = accessibility_pdf_path.read_bytes()
        if (
            accessibility_manifest["fingerprints"]["pdf_sha256"]
            != hashlib.sha256(accessibility_pdf).hexdigest()
            or accessibility_manifest["pdf"]["byte_length"] != len(accessibility_pdf)
        ):
            raise ValidationFailure("private 1.4 accessibility PDF hash/length drifted")
        structure = accessibility_manifest["structure"]
        if [node["structure_node_id"] for node in structure] != list(range(len(structure))):
            raise ValidationFailure("private 1.4 StructureNodeIds are not dense")
        for node in structure:
            parent = node["parent"]
            if parent is None:
                if node["structure_node_id"] != 0 or node["role"] != "Document":
                    raise ValidationFailure("private 1.4 structure root drifted")
            elif node["structure_node_id"] not in structure[parent]["children"]:
                raise ValidationFailure("private 1.4 structure parent/child closure drifted")
        marked = accessibility_manifest["marked_content"]
        selected_paint_ids = [
            paint_id
            for record in marked["records"]
            for paint_id in record["selected_paint_ids"]
        ]
        if selected_paint_ids != list(range(len(selected_paint_ids))):
            raise ValidationFailure("private 1.4 selected paint IDs are not dense")
        for page in marked["pages"]:
            mcids = [
                record["owner"]["mcid"]
                for record in marked["records"]
                if record["page_index"] == page["page_index"]
                and record["owner"]["kind"] == "structure"
            ]
            if mcids != list(range(len(mcids))) or len(mcids) != page["marked_content_count"]:
                raise ValidationFailure("private 1.4 page-local MCIDs are not dense")
        required_roles = {
            "Caption", "Document", "Em", "Exercise", "Figure", "Formula",
            "H1", "H2", "H3", "H4", "H5", "H6", "L", "LBody", "LI",
            "Lbl", "Link", "Note", "P", "Proof", "Reference", "Result",
            "Span", "Strong", "TBody", "TD", "TH", "THead", "TR", "Table",
        }
        if not required_roles.issubset({node["role"] for node in structure}):
            raise ValidationFailure("private 1.4 accessibility role coverage is incomplete")
        if {
            node["list_numbering"] for node in structure if node["role"] == "L"
        } != {"decimal", "disc"}:
            raise ValidationFailure("private 1.4 accessibility List coverage is incomplete")
        if accessibility_manifest["validators"] != [
            "typaxis.tagged-pdf-validator/1",
            "verapdf-greenfield/1.30.2:ua1",
            "typaxis.matterhorn-assessment/1",
        ]:
            raise ValidationFailure("private 1.4 accessibility validators drifted")
        unknown_accessibility_role = copy.deepcopy(accessibility_manifest)
        unknown_accessibility_role["structure"][0]["role"] = "Unknown"
        if not schema_errors(
            private_m4_validators["machine-accessibility-manifest.schema.json"],
            unknown_accessibility_role,
        ):
            raise ValidationFailure("private 1.4 accessibility manifest accepted an unknown role")
        extra_accessibility_member = copy.deepcopy(accessibility_manifest)
        extra_accessibility_member["tagged"] = True
        if not schema_errors(
            private_m4_validators["machine-accessibility-manifest.schema.json"],
            extra_accessibility_member,
        ):
            raise ValidationFailure("private 1.4 accessibility manifest accepted an extra member")

        m4_config = load_instance(MINIMAL_DIR / "typaxis.toml")
        m4_config["contract"] = "typaxis.contract/1.4"
        m4_limit_defaults = {
            "max_vector_nodes": 100_000,
            "max_vector_path_segments": 1_000_000,
            "max_vector_nesting_depth": 32,
            "max_math_layout_units": 1_000_000,
        }
        m4_config["limits"].update(m4_limit_defaults)
        m4_config_errors = schema_errors(
            private_m4_validators["package-config.schema.json"], m4_config
        )
        if m4_config_errors:
            raise ValidationFailure(
                "private 1.4 package config rejected M4 limit defaults: "
                + " | ".join(m4_config_errors)
            )
        if not schema_errors(
            versioned_current_validators["package-config.schema.json"], m4_config
        ):
            raise ValidationFailure("versioned 1.3 package config accepted private M4 limits")
        for limit_name, maximum in (
            ("max_vector_nodes", 1_000_000),
            ("max_vector_path_segments", 10_000_000),
            ("max_vector_nesting_depth", 64),
            ("max_math_layout_units", 10_000_000),
        ):
            for invalid in (0, maximum + 1):
                invalid_config = copy.deepcopy(m4_config)
                invalid_config["limits"][limit_name] = invalid
                if not schema_errors(
                    private_m4_validators["package-config.schema.json"], invalid_config
                ):
                    raise ValidationFailure(
                        f"private 1.4 accepted {limit_name}={invalid}"
                    )

        effective_config = load_instance(MINIMAL_DIR / "typaxis.toml")
        advanced_fixture_roots = (
            ("header/footer", STAGING_HEADER_FOOTER_FIXTURE_ROOT),
            ("columns", STAGING_COLUMNS_FIXTURE_ROOT),
            ("float", STAGING_FLOAT_FIXTURE_ROOT),
        )
        for fixture_label, fixture_root in advanced_fixture_roots:
            for fixture_name in ("combined", "empty", "oversize"):
                advanced_document_path = (
                    fixture_root / fixture_name / "job" / "document-package.json"
                )
                advanced_document = load_json(advanced_document_path)
                advanced_document_errors = schema_errors(
                    versioned_current_validators["document-package.schema.json"],
                    advanced_document,
                )
                if advanced_document_errors:
                    raise ValidationFailure(
                        f"the versioned 1.3 {fixture_label} {fixture_name} fixture was rejected: "
                        + " | ".join(advanced_document_errors)
                    )
            advanced_manifest_path = (
                fixture_root / "combined" / "staging-advanced-pagination.json"
            )
            advanced_manifest = load_json(advanced_manifest_path)
            advanced_manifest_errors = schema_errors(
                versioned_current_validators[
                    "machine-advanced-pagination-manifest.schema.json"
                ],
                advanced_manifest,
            )
            if advanced_manifest_errors:
                raise ValidationFailure(
                    f"the versioned 1.3 {fixture_label} advanced-pagination projection was rejected: "
                    + " | ".join(advanced_manifest_errors)
                )
            if advanced_manifest_path.read_bytes().rstrip(b"\n") != jcs_bytes(
                advanced_manifest
            ):
                raise ValidationFailure(
                    f"the versioned 1.3 {fixture_label} projection is not canonical JCS"
                )
        minimal_document = load_json(MINIMAL_DIR / "document-package.json")
        minimal_display = load_json(MINIMAL_DIR / "display-list.json")
        minimal_trace = load_json(MINIMAL_DIR / "layout-trace.json")
        minimal_manifest = load_json(MINIMAL_DIR / "build-manifest.json")
        staging_epoch = {
            "admitted_resources_sha256": "1" * 64,
            "document_sha256": "2" * 64,
            "resolved_input_sha256": "3" * 64,
            "style_page_master_sha256": "4" * 64,
        }
        staging_trace = {
            "algorithm": "typaxis.multi-flow-trace-facts/1",
            "contract": "typaxis.contract/1.2",
            "flow_positions": [
                {
                    "block_child_path": [0],
                    "child_flow_id": None,
                    "content_kind": "paragraph",
                    "content_owner_node_id": 1,
                    "epoch": staging_epoch,
                    "flow_id": 0,
                    "flow_local_ordinal": 0,
                    "owner_local_boundary": 0,
                    "owner_node_id": 0,
                    "parent_flow_id": None,
                    "terminal": False,
                },
                {
                    "block_child_path": [1, 0],
                    "child_flow_id": 1,
                    "content_kind": "list_item",
                    "content_owner_node_id": 3,
                    "epoch": staging_epoch,
                    "flow_id": 0,
                    "flow_local_ordinal": 1,
                    "owner_local_boundary": 0,
                    "owner_node_id": 0,
                    "parent_flow_id": None,
                    "terminal": False,
                },
                {
                    "block_child_path": [2],
                    "child_flow_id": None,
                    "content_kind": "page_break",
                    "content_owner_node_id": 8,
                    "epoch": staging_epoch,
                    "flow_id": 0,
                    "flow_local_ordinal": 2,
                    "owner_local_boundary": 0,
                    "owner_node_id": 0,
                    "parent_flow_id": None,
                    "terminal": False,
                },
                {
                    "block_child_path": [],
                    "child_flow_id": None,
                    "content_kind": None,
                    "content_owner_node_id": None,
                    "epoch": staging_epoch,
                    "flow_id": 0,
                    "flow_local_ordinal": 3,
                    "owner_local_boundary": 0,
                    "owner_node_id": 0,
                    "parent_flow_id": None,
                    "terminal": True,
                },
                {
                    "block_child_path": [1, 0, 0],
                    "child_flow_id": None,
                    "content_kind": "paragraph",
                    "content_owner_node_id": 4,
                    "epoch": staging_epoch,
                    "flow_id": 1,
                    "flow_local_ordinal": 0,
                    "owner_local_boundary": 0,
                    "owner_node_id": 3,
                    "parent_flow_id": 0,
                    "terminal": False,
                },
                {
                    "block_child_path": [1, 0, 1, 0],
                    "child_flow_id": 2,
                    "content_kind": "list_item",
                    "content_owner_node_id": 6,
                    "epoch": staging_epoch,
                    "flow_id": 1,
                    "flow_local_ordinal": 1,
                    "owner_local_boundary": 0,
                    "owner_node_id": 3,
                    "parent_flow_id": 0,
                    "terminal": False,
                },
                {
                    "block_child_path": [1, 0],
                    "child_flow_id": None,
                    "content_kind": None,
                    "content_owner_node_id": None,
                    "epoch": staging_epoch,
                    "flow_id": 1,
                    "flow_local_ordinal": 2,
                    "owner_local_boundary": 0,
                    "owner_node_id": 3,
                    "parent_flow_id": 0,
                    "terminal": True,
                },
                {
                    "block_child_path": [1, 0, 1, 0, 0],
                    "child_flow_id": None,
                    "content_kind": "paragraph",
                    "content_owner_node_id": 7,
                    "epoch": staging_epoch,
                    "flow_id": 2,
                    "flow_local_ordinal": 0,
                    "owner_local_boundary": 0,
                    "owner_node_id": 6,
                    "parent_flow_id": 1,
                    "terminal": False,
                },
                {
                    "block_child_path": [1, 0, 1, 0],
                    "child_flow_id": None,
                    "content_kind": None,
                    "content_owner_node_id": None,
                    "epoch": staging_epoch,
                    "flow_id": 2,
                    "flow_local_ordinal": 1,
                    "owner_local_boundary": 0,
                    "owner_node_id": 6,
                    "parent_flow_id": 1,
                    "terminal": True,
                },
            ],
            "flow_registry_sha256": "5" * 64,
            "selected_state_sha256": "6" * 64,
        }
        staging_manifest = {
            "contract": "typaxis.contract/1.2",
            "flow_registry_sha256": "5" * 64,
            "flows": [
                {
                    "flow_id": 0,
                    "owner_node_id": 0,
                    "parent_flow_id": None,
                    "terminal": 3,
                },
                {
                    "flow_id": 1,
                    "owner_node_id": 3,
                    "parent_flow_id": 0,
                    "terminal": 2,
                },
                {
                    "flow_id": 2,
                    "owner_node_id": 6,
                    "parent_flow_id": 1,
                    "terminal": 1,
                },
            ],
            "selected_state_sha256": "6" * 64,
        }
        staging_style_document = load_json(
            STAGING_STYLE_FIXTURE_DIR / "job" / "document-package.json"
        )
        staging_style_selected = load_json(
            STAGING_STYLE_FIXTURE_DIR / "staging-selected-state.json"
        )
        staging_style_expectation = load_json(
            STAGING_STYLE_FIXTURE_DIR / "staging-expectation.json"
        )
        staging_list_document = load_json(
            STAGING_LIST_FIXTURE_DIR / "job" / "document-package.json"
        )
        staging_list_selected = load_json(
            STAGING_LIST_FIXTURE_DIR / "staging-selected-state.json"
        )
        staging_list_expectation = load_json(
            STAGING_LIST_FIXTURE_DIR / "staging-expectation.json"
        )
        staging_page_break_document = load_json(
            STAGING_PAGE_BREAK_FIXTURE_DIR / "job" / "document-package.json"
        )
        staging_page_break_selected = load_json(
            STAGING_PAGE_BREAK_FIXTURE_DIR / "staging-selected-state.json"
        )
        staging_page_break_trace = load_json(
            STAGING_PAGE_BREAK_FIXTURE_DIR / "staging-trace.json"
        )
        staging_page_break_expectation = load_json(
            STAGING_PAGE_BREAK_FIXTURE_DIR / "staging-expectation.json"
        )
        staging_figure_document = load_json(
            STAGING_FIGURE_FIXTURE_DIR / "job" / "document-package.json"
        )
        staging_figure_selected = load_json(
            STAGING_FIGURE_FIXTURE_DIR / "staging-selected-state.json"
        )
        staging_figure_expectation = load_json(
            STAGING_FIGURE_FIXTURE_DIR / "staging-expectation.json"
        )
        staging_figure_png_hex = (
            STAGING_FIGURE_FIXTURE_DIR / "job" / "figure.data.hex"
        ).read_text(encoding="ascii").strip()
        try:
            staging_figure_png = bytes.fromhex(staging_figure_png_hex)
        except ValueError as error:
            raise ValidationFailure("MI2-06 PNG fixture hex is invalid") from error
        if staging_figure_png.hex() != staging_figure_png_hex:
            raise ValidationFailure("MI2-06 PNG fixture hex is not canonical lowercase")
        staging_link_document = load_json(
            STAGING_LINK_FIXTURE_DIR / "job" / "document-package.json"
        )
        staging_link_selected = load_json(
            STAGING_LINK_FIXTURE_DIR / "staging-selected-state.json"
        )
        staging_link_expectation = load_json(
            STAGING_LINK_FIXTURE_DIR / "staging-expectation.json"
        )
        staging_link_font_hex = (
            STAGING_LINK_FIXTURE_DIR / "job" / "body.ttf.hex"
        ).read_text(encoding="ascii").strip()
        try:
            staging_link_font = bytes.fromhex(staging_link_font_hex)
        except ValueError as error:
            raise ValidationFailure("MI2-07 font fixture hex is invalid") from error
        if staging_link_font.hex() != staging_link_font_hex:
            raise ValidationFailure("MI2-07 font fixture hex is not canonical lowercase")
        validate_staging_multi_flow_bundle(staging_trace, staging_manifest)
        style_document_errors = schema_errors(
            staging_validators["document-package.schema.json"], staging_style_document
        )
        if style_document_errors:
            raise ValidationFailure(
                "versioned 1.2 DocumentPackage rejected typed block styles: "
                + " | ".join(style_document_errors)
            )
        if not schema_errors(
            previous_validators["document-package.schema.json"], staging_style_document
        ):
            raise ValidationFailure(
                "frozen 1.1 DocumentPackage Schema accepted contract 1.2 styles"
            )
        selected_errors = schema_errors(
            staging_validators["machine-block-style-manifest.schema.json"],
            staging_style_selected,
        )
        if selected_errors:
            raise ValidationFailure(
                "versioned 1.2 typed-style selected state was rejected: "
                + " | ".join(selected_errors)
            )

        list_document_errors = schema_errors(
            staging_validators["document-package.schema.json"], staging_list_document
        )
        if list_document_errors:
            raise ValidationFailure(
                "versioned 1.2 DocumentPackage rejected machine lists: "
                + " | ".join(list_document_errors)
            )
        if not schema_errors(
            previous_validators["document-package.schema.json"], staging_list_document
        ):
            raise ValidationFailure(
                "frozen 1.1 DocumentPackage Schema accepted the MI2-04 package"
            )
        list_selected_errors = schema_errors(
            staging_validators["machine-list-manifest.schema.json"],
            staging_list_selected,
        )
        if list_selected_errors:
            raise ValidationFailure(
                "versioned 1.2 machine-list selected state was rejected: "
                + " | ".join(list_selected_errors)
            )
        if (
            STAGING_LIST_FIXTURE_DIR / "staging-selected-state.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_list_selected):
            raise ValidationFailure("MI2-04 selected-state golden is not canonical JCS")
        validate_staging_machine_list_bundle(
            staging_list_document, staging_list_selected, staging_list_expectation
        )

        wrong_marker_owner = copy.deepcopy(staging_list_selected)
        wrong_marker_owner["items"][0]["marker_key"]["owner"] = 7
        try:
            validate_staging_machine_list_bundle(
                staging_list_document, wrong_marker_owner, staging_list_expectation
            )
        except ValidationFailure:
            pass
        else:
            raise ValidationFailure("MI2-04 semantic validator accepted a wrong marker owner")

        orphan_marker = copy.deepcopy(staging_list_selected)
        orphan_marker["items"][0]["first_line_fragment_id"] = 1
        try:
            validate_staging_machine_list_bundle(
                staging_list_document, orphan_marker, staging_list_expectation
            )
        except ValidationFailure:
            pass
        else:
            raise ValidationFailure("MI2-04 semantic validator accepted a marker orphan")

        page_break_document_errors = schema_errors(
            staging_validators["document-package.schema.json"],
            staging_page_break_document,
        )
        if page_break_document_errors:
            raise ValidationFailure(
                "versioned 1.2 DocumentPackage rejected forced page breaks: "
                + " | ".join(page_break_document_errors)
            )
        if not schema_errors(
            previous_validators["document-package.schema.json"], staging_page_break_document
        ):
            raise ValidationFailure(
                "frozen 1.1 DocumentPackage Schema accepted the MI2-05 package"
            )
        page_break_selected_errors = schema_errors(
            staging_validators["machine-forced-page-break-manifest.schema.json"],
            staging_page_break_selected,
        )
        if page_break_selected_errors:
            raise ValidationFailure(
                "versioned 1.2 forced-page-break selected state was rejected: "
                + " | ".join(page_break_selected_errors)
            )
        page_break_trace_errors = schema_errors(
            staging_validators["machine-forced-page-break-trace.schema.json"],
            staging_page_break_trace,
        )
        if page_break_trace_errors:
            raise ValidationFailure(
                "versioned 1.2 forced-page-break trace was rejected: "
                + " | ".join(page_break_trace_errors)
            )
        if (
            STAGING_PAGE_BREAK_FIXTURE_DIR / "staging-selected-state.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_page_break_selected):
            raise ValidationFailure("MI2-05 selected-state golden is not canonical JCS")
        if (
            STAGING_PAGE_BREAK_FIXTURE_DIR / "staging-trace.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_page_break_trace):
            raise ValidationFailure("MI2-05 trace golden is not canonical JCS")
        validate_staging_forced_page_break_bundle(
            staging_page_break_document,
            staging_page_break_trace,
            staging_page_break_selected,
            staging_page_break_expectation,
        )

        stale_cursor = copy.deepcopy(staging_page_break_selected)
        stale_cursor["forced_page_breaks"][0]["after_cursor"] = copy.deepcopy(
            stale_cursor["forced_page_breaks"][0]["before_cursor"]
        )
        try:
            validate_staging_forced_page_break_bundle(
                staging_page_break_document,
                staging_page_break_trace,
                stale_cursor,
                staging_page_break_expectation,
            )
        except ValidationFailure:
            pass
        else:
            raise ValidationFailure("MI2-05 semantic validator accepted a stale cursor")

        figure_document_errors = schema_errors(
            staging_validators["document-package.schema.json"], staging_figure_document
        )
        if figure_document_errors:
            raise ValidationFailure(
                "versioned 1.2 DocumentPackage rejected the PNG figure: "
                + " | ".join(figure_document_errors)
            )
        if not schema_errors(
            previous_validators["document-package.schema.json"], staging_figure_document
        ):
            raise ValidationFailure(
                "frozen 1.1 DocumentPackage Schema accepted the MI2-06 package"
            )
        figure_selected_errors = schema_errors(
            staging_validators["machine-figure-manifest.schema.json"],
            staging_figure_selected,
        )
        if figure_selected_errors:
            raise ValidationFailure(
                "versioned 1.2 machine-figure selected state was rejected: "
                + " | ".join(figure_selected_errors)
            )
        if not schema_errors(
            previous_validators["build-manifest.schema.json"], staging_figure_selected
        ):
            raise ValidationFailure(
                "frozen 1.1 manifest Schema accepted MI2-06 versioned facts"
            )
        if (
            STAGING_FIGURE_FIXTURE_DIR / "staging-selected-state.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_figure_selected):
            raise ValidationFailure("MI2-06 selected-state golden is not canonical JCS")
        validate_staging_machine_figure_bundle(
            staging_figure_document,
            staging_figure_selected,
            staging_figure_expectation,
            staging_figure_png,
        )

        declared_media = copy.deepcopy(staging_figure_document)
        declared_media["resources"]["images"][0]["media_kind"] = "png"
        if not schema_errors(
            staging_validators["document-package.schema.json"], declared_media
        ):
            raise ValidationFailure("MI2-06 Schema accepted caller-declared image media")

        for label, mutate in (
            (
                "missing",
                lambda facts: facts.__setitem__("image_xobjects", []),
            ),
            (
                "extra",
                lambda facts: facts["image_xobjects"].append(
                    {"image_id": 1, "resource_name": "/Im1"}
                ),
            ),
            (
                "wrong",
                lambda facts: facts["image_xobjects"][0].__setitem__("image_id", 1),
            ),
        ):
            tampered_xobject = copy.deepcopy(staging_figure_selected)
            mutate(tampered_xobject)
            try:
                validate_staging_machine_figure_bundle(
                    staging_figure_document,
                    tampered_xobject,
                    staging_figure_expectation,
                    staging_figure_png,
                )
            except ValidationFailure:
                pass
            else:
                raise ValidationFailure(
                    f"MI2-06 semantic validator accepted {label} image-XObject closure"
                )

        link_document_errors = schema_errors(
            staging_validators["document-package.schema.json"], staging_link_document
        )
        if link_document_errors:
            raise ValidationFailure(
                "versioned 1.2 DocumentPackage rejected machine links: "
                + " | ".join(link_document_errors)
            )
        if not schema_errors(
            previous_validators["document-package.schema.json"], staging_link_document
        ):
            raise ValidationFailure(
                "frozen 1.1 DocumentPackage Schema accepted the MI2-07 package"
            )
        link_selected_errors = schema_errors(
            staging_validators["machine-link-manifest.schema.json"],
            staging_link_selected,
        )
        if link_selected_errors:
            raise ValidationFailure(
                "versioned 1.2 machine-link selected state was rejected: "
                + " | ".join(link_selected_errors)
            )
        if not schema_errors(
            previous_validators["build-manifest.schema.json"], staging_link_selected
        ):
            raise ValidationFailure(
                "frozen 1.1 manifest Schema accepted MI2-07 versioned facts"
            )
        if (
            STAGING_LINK_FIXTURE_DIR / "staging-selected-state.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_link_selected):
            raise ValidationFailure("MI2-07 selected-state golden is not canonical JCS")
        validate_staging_machine_link_bundle(
            staging_link_document,
            staging_link_selected,
            staging_link_expectation,
            staging_link_font,
        )

        link_tampers = (
            ("missing", lambda facts: facts["links"][0]["rectangles"].clear()),
            (
                "extra",
                lambda facts: facts["links"][0]["rectangles"].append(
                    copy.deepcopy(facts["links"][0]["rectangles"][0])
                ),
            ),
            (
                "wrong page",
                lambda facts: facts["links"][0]["rectangles"][0].__setitem__(
                    "page_index", 1
                ),
            ),
            (
                "wrong target",
                lambda facts: facts["links"][0]["target"].__setitem__(
                    "anchor_id", "other"
                ),
            ),
            (
                "rectangle",
                lambda facts: facts["links"][0]["rectangles"][0]["rect"].__setitem__(
                    "x", facts["pages"][0]["width"]
                ),
            ),
        )
        for label, mutate in link_tampers:
            tampered_link = copy.deepcopy(staging_link_selected)
            mutate(tampered_link)
            try:
                validate_staging_machine_link_bundle(
                    staging_link_document,
                    tampered_link,
                    staging_link_expectation,
                    staging_link_font,
                )
            except ValidationFailure:
                pass
            else:
                raise ValidationFailure(
                    f"MI2-07 semantic validator accepted {label} annotation closure tamper"
                )

        expected_properties = {
            "space_before",
            "space_after",
            "start_indent",
            "end_indent",
            "text_align",
            "width",
            "keep_with_next",
            "keep_caption",
        }
        coverage = staging_style_expectation.get("property_coverage", [])
        covered_properties = {row.get("property") for row in coverage}
        if len(coverage) != 8 or covered_properties != expected_properties:
            raise ValidationFailure(
                "MI2-03 property coverage is missing, duplicated, or advertises an unused property"
            )
        for row in coverage:
            if any(not row.get(axis) for axis in ("consumer", "display", "pdf", "manifest")):
                raise ValidationFailure(
                    f"MI2-03 property lacks a complete observation chain: {row.get('property')}"
                )
        scenarios = staging_style_expectation.get("scenarios", {})
        if scenarios.get("minimum") != 0 or scenarios.get("exact_max") != JSON_SAFE_INTEGER_MAX:
            raise ValidationFailure("MI2-03 exact length boundary fixtures drifted")
        if scenarios.get("max_plus_one") != JSON_SAFE_INTEGER_MAX + 1:
            raise ValidationFailure("MI2-03 max+1 fixture drifted")
        if scenarios.get("unused_advertised_property") is not False:
            raise ValidationFailure("MI2-03 claims an unused advertised property")
        if scenarios.get("page_split") is not True or not scenarios.get(
            "pdf_content_observation"
        ):
            raise ValidationFailure("MI2-03 page-split/PDF observation fixture is incomplete")

        declarations = staging_style_document["style_sheet"]["rules"][0]["declarations"]
        by_name = {declaration["name"]: declaration for declaration in declarations}
        if by_name["space_before"]["value"]["value"] != 0:
            raise ValidationFailure("MI2-03 minimum length declaration drifted")
        if by_name["space_after"]["value"]["value"] != JSON_SAFE_INTEGER_MAX:
            raise ValidationFailure("MI2-03 maximum length declaration drifted")
        if staging_style_document["style_sheet"]["rules"][1]["extends"] != "paragraph-base":
            raise ValidationFailure("MI2-03 extends/override fixture drifted")

        invalid_max = copy.deepcopy(staging_style_document)
        invalid_max["style_sheet"]["rules"][0]["declarations"][1]["value"][
            "value"
        ] = JSON_SAFE_INTEGER_MAX + 1
        if not schema_errors(
            staging_validators["document-package.schema.json"], invalid_max
        ):
            raise ValidationFailure("versioned 1.2 Schema accepted typed length max+1")
        invalid_tag = copy.deepcopy(staging_style_document)
        invalid_tag["style_sheet"]["rules"][0]["declarations"][0]["value"][
            "kind"
        ] = "integer"
        if not schema_errors(
            staging_validators["document-package.schema.json"], invalid_tag
        ):
            raise ValidationFailure("versioned 1.2 Schema accepted a wrong property tag")
        invalid_unknown = copy.deepcopy(staging_style_document)
        invalid_unknown["style_sheet"]["rules"][0]["declarations"][0][
            "name"
        ] = "future_property"
        if not schema_errors(
            staging_validators["document-package.schema.json"], invalid_unknown
        ):
            raise ValidationFailure("versioned 1.2 Schema accepted an unknown property")

        source_bytes = (STAGING_STYLE_FIXTURE_DIR / "job" / "input.tsf").read_bytes()
        source_declaration = staging_style_document["sources"][0]
        if (
            len(source_bytes) != source_declaration["utf8_byte_length"]
            or hashlib.sha256(source_bytes).hexdigest() != source_declaration["sha256"]
        ):
            raise ValidationFailure("MI2-03 companion source bytes do not match the package")
        list_source_bytes = (STAGING_LIST_FIXTURE_DIR / "job" / "input.tsf").read_bytes()
        list_source_declaration = staging_list_document["sources"][0]
        if (
            len(list_source_bytes) != list_source_declaration["utf8_byte_length"]
            or hashlib.sha256(list_source_bytes).hexdigest()
            != list_source_declaration["sha256"]
        ):
            raise ValidationFailure("MI2-04 companion source bytes do not match the package")
        page_break_source_bytes = (
            STAGING_PAGE_BREAK_FIXTURE_DIR / "job" / "input.tsf"
        ).read_bytes()
        page_break_source_declaration = staging_page_break_document["sources"][0]
        if (
            len(page_break_source_bytes)
            != page_break_source_declaration["utf8_byte_length"]
            or hashlib.sha256(page_break_source_bytes).hexdigest()
            != page_break_source_declaration["sha256"]
        ):
            raise ValidationFailure("MI2-05 companion source bytes do not match the package")
        figure_source_bytes = (
            STAGING_FIGURE_FIXTURE_DIR / "job" / "input.tsf"
        ).read_bytes()
        figure_source_declaration = staging_figure_document["sources"][0]
        if (
            len(figure_source_bytes)
            != figure_source_declaration["utf8_byte_length"]
            or hashlib.sha256(figure_source_bytes).hexdigest()
            != figure_source_declaration["sha256"]
        ):
            raise ValidationFailure("MI2-06 companion source bytes do not match the package")
        if (
            STAGING_FIGURE_FIXTURE_DIR / "job" / "document-package.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_figure_document):
            raise ValidationFailure("MI2-06 DocumentPackage fixture is not canonical JCS")
        link_source_bytes = (STAGING_LINK_FIXTURE_DIR / "job" / "input.tsf").read_bytes()
        link_source_declaration = staging_link_document["sources"][0]
        if (
            len(link_source_bytes) != link_source_declaration["utf8_byte_length"]
            or hashlib.sha256(link_source_bytes).hexdigest()
            != link_source_declaration["sha256"]
        ):
            raise ValidationFailure("MI2-07 companion source bytes do not match the package")
        if (
            STAGING_LINK_FIXTURE_DIR / "job" / "document-package.json"
        ).read_bytes().rstrip(b"\n") != jcs_bytes(staging_link_document):
            raise ValidationFailure("MI2-07 DocumentPackage fixture is not canonical JCS")
        jcs_golden_count = validate_jcs_golden(effective_config)
        machine_expectation_count, machine_matrix_count = validate_machine_fixture_bundle(
            validators
        )

        compatibility_metadata = load_json(
            COMPATIBILITY_DIR / "document-package-1.0-canonical.json"
        )
        compatibility_document = load_json(
            COMPATIBILITY_DIR / compatibility_metadata["fixture"]
        )
        frozen_errors = schema_errors(
            frozen_validators["document-package.schema.json"], compatibility_document
        )
        if frozen_errors:
            raise ValidationFailure(
                "the frozen 1.0 DocumentPackage fixture was rejected: "
                + " | ".join(frozen_errors)
            )
        if not schema_errors(
            validators["document-package.schema.json"], compatibility_document
        ):
            raise ValidationFailure("the 1.0 DocumentPackage was registered as current 1.3")
        compatibility_jcs = jcs_bytes(compatibility_document)
        if (
            compatibility_document.get("contract") != "typaxis.contract/1.0"
            or compatibility_metadata != {
                "algorithm": "rfc8785-jcs-sha256/1",
                "canonical_sha256": hashlib.sha256(compatibility_jcs).hexdigest(),
                "contract": "typaxis.contract/1.0",
                "fixture": "document-package-1.0.json",
            }
        ):
            raise ValidationFailure(
                "the 1.0 DocumentPackage canonical hash does not retain its contract field"
            )

        additive_current_shapes = {
            "build-manifest.schema.json": minimal_manifest,
            "diagnostics.schema.json": load_json(MINIMAL_DIR / "diagnostics.json"),
            "package-config.schema.json": effective_config,
        }
        for schema_name, current_shape in additive_current_shapes.items():
            disguised_current = copy.deepcopy(current_shape)
            disguised_current["contract"] = "typaxis.contract/1.0"
            if not schema_errors(frozen_validators[schema_name], disguised_current):
                raise ValidationFailure(
                    f"the frozen 1.0 {schema_name} consumer accepted its additive 1.1 shape"
                )

        for path, schema_name in POSITIVE_FIXTURES.items():
            instance = materialize_patch_fixture(load_instance(path), schema_name, str(path))
            errors = schema_errors(validators[schema_name], instance)
            if errors:
                raise ValidationFailure(
                    f"{path}: positive fixture rejected by {schema_name}: " + " | ".join(errors)
                )
            semantic_rules = conformance_rule_ids(schema_name, instance, effective_config)
            if semantic_rules:
                raise ValidationFailure(
                    f"{path}: positive fixture violates conformance rules {sorted(semantic_rules)}"
                )

        base_cross_rules = cross_artifact_rule_ids(
            effective_config, minimal_document, minimal_display,
            minimal_trace, minimal_manifest, MINIMAL_DIR
        )
        if base_cross_rules:
            raise ValidationFailure(
                "minimal artifacts violate cross-artifact rules "
                f"{sorted(base_cross_rules)}"
            )

        for path in POSITIVE_CROSS_FIXTURES:
            positive_bundle = materialize_cross_fixture(
                load_json(path), effective_config, minimal_document, minimal_display,
                minimal_trace, minimal_manifest, str(path)
            )
            for schema_name, instance in zip(
                (
                    "package-config.schema.json", "document-package.schema.json",
                    "display-list.schema.json", "layout-trace.schema.json",
                    "build-manifest.schema.json",
                ),
                positive_bundle,
            ):
                errors = schema_errors(validators[schema_name], instance)
                semantic_rules = conformance_rule_ids(
                    schema_name, instance, positive_bundle[0]
                )
                if errors or semantic_rules:
                    raise ValidationFailure(
                        f"{path}: positive cross fixture violates {schema_name}: "
                        f"schema={errors}, semantic={sorted(semantic_rules)}"
                    )
            positive_cross_rules = cross_artifact_rule_ids(
                *positive_bundle, MINIMAL_DIR
            )
            if positive_cross_rules:
                raise ValidationFailure(
                    f"{path}: positive cross fixture violates "
                    f"{sorted(positive_cross_rules)}"
                )

        expected_path = INVALID_DIR / "expected-errors.json"
        expected = load_json(expected_path)
        indexed_paths = {INVALID_DIR / name for name in expected}
        discovered_paths = {
            *INVALID_DIR.glob("*.json"),
            *INVALID_DIR.glob("*.toml"),
        } - {expected_path}
        if indexed_paths != discovered_paths:
            missing = sorted(str(path.name) for path in discovered_paths - indexed_paths)
            stale = sorted(str(path.name) for path in indexed_paths - discovered_paths)
            raise ValidationFailure(
                f"invalid fixture index mismatch; missing={missing}, stale={stale}"
            )

        for name, expectation in expected.items():
            path = INVALID_DIR / name
            if set(expectation) != {"rule_id", "schema_rejects"}:
                raise ValidationFailure(f"{name}: expectation must contain rule_id and schema_rejects")
            rule_id = expectation["rule_id"]
            schema_rejects = expectation["schema_rejects"]
            if not isinstance(rule_id, str) or RULE_ID.fullmatch(rule_id) is None:
                raise ValidationFailure(f"{name}: malformed conformance rule_id")
            if type(schema_rejects) is not bool:
                raise ValidationFailure(f"{name}: schema_rejects must be boolean")

            if name.startswith("cross-"):
                if schema_rejects:
                    raise ValidationFailure(
                        f"{name}: cross-artifact fixtures must set schema_rejects=false"
                    )
                (
                    cross_config, cross_document, cross_display,
                    cross_trace, cross_manifest,
                ) = materialize_cross_fixture(
                    load_json(path),
                    effective_config,
                    minimal_document,
                    minimal_display,
                    minimal_trace,
                    minimal_manifest,
                    str(path),
                )
                cross_instances = (
                    ("package-config.schema.json", cross_config),
                    ("document-package.schema.json", cross_document),
                    ("display-list.schema.json", cross_display),
                    ("layout-trace.schema.json", cross_trace),
                    ("build-manifest.schema.json", cross_manifest),
                )
                for cross_schema, cross_instance in cross_instances:
                    errors = schema_errors(validators[cross_schema], cross_instance)
                    if errors:
                        raise ValidationFailure(
                            f"{name}: cross fixture is not independently valid under "
                            f"{cross_schema}: " + " | ".join(errors)
                        )
                    standalone_rules = conformance_rule_ids(
                        cross_schema, cross_instance, cross_config
                    )
                    depth_preflight_only = (
                        rule_id == "CROSS_LIMIT_AST_NESTING_DEPTH"
                        and cross_schema == "document-package.schema.json"
                        and standalone_rules == {rule_id}
                    )
                    if standalone_rules and not depth_preflight_only:
                        raise ValidationFailure(
                            f"{name}: cross fixture also violates standalone rules "
                            f"{sorted(standalone_rules)}"
                        )
                semantic_rules = cross_artifact_rule_ids(
                    cross_config, cross_document, cross_display,
                    cross_trace, cross_manifest, MINIMAL_DIR
                )
                if semantic_rules != {rule_id}:
                    raise ValidationFailure(
                        f"{name}: expected only conformance rule {rule_id}, "
                        f"observed {sorted(semantic_rules)}"
                    )
                continue

            schema_name = next(
                (schema for prefix, schema in INVALID_SCHEMA_BY_PREFIX.items() if name.startswith(prefix)),
                None,
            )
            if schema_name is None:
                raise ValidationFailure(f"{name}: no schema mapping for invalid fixture")
            instance = materialize_patch_fixture(
                load_instance(path), schema_name, str(path)
            )
            errors = schema_errors(validators[schema_name], instance)
            if bool(errors) != schema_rejects:
                details = " | ".join(errors) if errors else "schema accepted fixture"
                raise ValidationFailure(
                    f"{name}: schema_rejects={schema_rejects} but observed {bool(errors)}: {details}"
                )
            semantic_rules = conformance_rule_ids(schema_name, instance, effective_config)
            if semantic_rules != {rule_id}:
                raise ValidationFailure(
                    f"{name}: expected only conformance rule {rule_id}, "
                    f"observed {sorted(semantic_rules)}"
                )

        config_digest = hashlib.sha256(jcs_bytes(effective_config)).hexdigest()
        manifest_cases = [(MINIMAL_DIR / "build-manifest.json", minimal_manifest)]
        for path in sorted(INVALID_DIR.glob("manifest-*.json")):
            manifest_cases.append(
                (
                    path,
                    materialize_patch_fixture(
                        load_json(path), "build-manifest.schema.json", str(path)
                    ),
                )
            )
        for path, manifest in manifest_cases:
            if manifest["config_sha256"] != config_digest:
                raise ValidationFailure(f"{path}: config_sha256 is not the effective-config JCS hash")

        for index, record in enumerate(minimal_manifest["inputs"]):
            verify_file_record(MINIMAL_DIR, record, f"minimal manifest input {index}")
        for index, record in enumerate(minimal_manifest["fonts"]):
            verify_file_record(MINIMAL_DIR, record, f"minimal manifest font {index}")
        for index, record in enumerate(minimal_manifest["images"]):
            verify_file_record(MINIMAL_DIR, record, f"minimal manifest image {index}")

        manifest_validator = validators["build-manifest.schema.json"]
        layout_summary = copy.deepcopy(minimal_manifest["layout"])
        output_record = copy.deepcopy(minimal_manifest["output"])
        manifest_state_cases = (
            ("built/full", "built", layout_summary, output_record, True),
            ("built/null-layout", "built", None, output_record, False),
            ("built/null-output", "built", layout_summary, None, False),
            ("failed/null", "failed", None, None, True),
            ("failed/layout", "failed", layout_summary, None, True),
            ("failed/output", "failed", None, output_record, False),
        )
        for label, status, layout, output, should_accept in manifest_state_cases:
            candidate = copy.deepcopy(minimal_manifest)
            candidate["status"] = status
            candidate["layout"] = copy.deepcopy(layout)
            candidate["output"] = copy.deepcopy(output)
            accepted = not schema_errors(manifest_validator, candidate)
            if accepted != should_accept:
                raise ValidationFailure(
                    f"manifest status conditional {label} acceptance was {accepted}"
                )

        raw_package_input = {
            "bytes": 1,
            "canonical_sha256": None,
            "contract": None,
            "profile_receipt_sha256": None,
            "sha256": "0" * 64,
            "uri": "document-package.json",
        }
        decoded_package_input = {
            **raw_package_input,
            "canonical_sha256": "1" * 64,
            "contract": "typaxis.contract/1.0",
            "profile_receipt_sha256": "2" * 64,
        }
        half_decoded_package_input = {
            **raw_package_input,
            "contract": "typaxis.contract/1.1",
        }
        manifest_input_cases = (
            ("reference/null", "typaxis.reference-source/1", "built", None, True),
            (
                "reference/package",
                "typaxis.reference-source/1",
                "built",
                decoded_package_input,
                False,
            ),
            (
                "machine/built-decoded",
                "typaxis.machine-pdf/paragraph-1",
                "built",
                decoded_package_input,
                True,
            ),
            (
                "machine/built-raw",
                "typaxis.machine-pdf/paragraph-1",
                "built",
                raw_package_input,
                False,
            ),
            (
                "machine/built-null",
                "typaxis.machine-pdf/paragraph-1",
                "built",
                None,
                False,
            ),
            (
                "machine/failed-null",
                "typaxis.machine-pdf/paragraph-1",
                "failed",
                None,
                True,
            ),
            (
                "machine/failed-raw",
                "typaxis.machine-pdf/paragraph-1",
                "failed",
                raw_package_input,
                True,
            ),
            (
                "machine/failed-decoded",
                "typaxis.machine-pdf/paragraph-1",
                "failed",
                decoded_package_input,
                True,
            ),
            (
                "machine/failed-half-decoded",
                "typaxis.machine-pdf/paragraph-1",
                "failed",
                half_decoded_package_input,
                False,
            ),
        )
        for label, profile, status, package_input, should_accept in manifest_input_cases:
            candidate = copy.deepcopy(minimal_manifest)
            candidate["input_profile"] = profile
            candidate["package_input"] = copy.deepcopy(package_input)
            candidate["status"] = status
            if status == "failed":
                candidate["layout"] = None
                candidate["output"] = None
            accepted = not schema_errors(manifest_validator, candidate)
            if accepted != should_accept:
                raise ValidationFailure(
                    f"manifest input conditional {label} acceptance was {accepted}"
                )

        policy_cases = (
            ("converged", "lowest_cost_then_earliest"),
            ("cycle_fallback", None),
            ("max_pass_fallback", None),
        )
        for status, fallback_policy in policy_cases:
            candidate = copy.deepcopy(minimal_manifest)
            candidate["layout"]["status"] = status
            candidate["layout"]["fallback_policy"] = fallback_policy
            if not schema_errors(manifest_validator, candidate):
                raise ValidationFailure(
                    f"manifest layout accepted invalid {status} fallback_policy"
                )

        stdout_manifest = copy.deepcopy(minimal_manifest)
        stdout_manifest["output"]["sink"] = "stdout"
        if schema_errors(manifest_validator, stdout_manifest):
            raise ValidationFailure("manifest rejected the host-independent stdout sink")

        for field in ("bytes", "page_count", "pdf_object_count"):
            candidate = copy.deepcopy(minimal_manifest)
            candidate["output"][field] = 0
            if not schema_errors(manifest_validator, candidate):
                raise ValidationFailure(f"built manifest accepted output.{field}=0")

        zero_input_manifest = copy.deepcopy(minimal_manifest)
        zero_input_manifest["inputs"].append(
            {"uri": "zero.tsf", "bytes": 0, "sha256": "0" * 64}
        )
        if schema_errors(manifest_validator, zero_input_manifest):
            raise ValidationFailure("manifest rejected a zero-byte input source record")

        positive_resource_records = {
            "fonts": {
                "font_face_id": 0,
                "uri": "font.bin",
                "face_index": 0,
                "bytes": 1,
                "sha256": "0" * 64,
                "units_per_em": 1000,
                "glyph_count": 1,
            },
            "images": {
                "image_id": 0,
                "uri": "image.bin",
                "bytes": 1,
                "sha256": "0" * 64,
                "attested_media_kind": "png",
                "pixel_width": 1,
                "pixel_height": 1,
                "decoded_bytes": 1,
            },
        }
        for collection, record in positive_resource_records.items():
            candidate = copy.deepcopy(minimal_manifest)
            candidate[collection].append(copy.deepcopy(record))
            if schema_errors(manifest_validator, candidate):
                raise ValidationFailure(
                    f"manifest rejected a positive-byte {collection} record"
                )
            candidate[collection][0]["bytes"] = 0
            if not schema_errors(manifest_validator, candidate):
                raise ValidationFailure(
                    f"manifest accepted a zero-byte {collection} record"
                )

        order_manifest = load_json(INVALID_DIR / "manifest-noncanonical-order.json")
        for index, record in enumerate(order_manifest["inputs"]):
            verify_file_record(INVALID_DIR, record, f"order fixture input {index}")

        print(
            "validated "
            f"{len(frozen_schemas)} frozen 1.0, {len(previous_schemas)} frozen 1.1, "
            f"{len(staging_schemas)} frozen 1.2, {len(schemas)} current 1.3 aliases, and "
            f"{len(versioned_current_schemas)} versioned 1.3 and "
            f"{len(private_m4_schemas)} private 1.4 schemas, "
            f"{frozen_reference_count + previous_reference_count + reference_count + staging_reference_count + versioned_current_reference_count + private_m4_reference_count} refs, "
            f"{len(POSITIVE_FIXTURES)} artifact and "
            f"{len(POSITIVE_CROSS_FIXTURES)} cross-bundle positive fixtures, "
            f"{len(expected)} exact-rule invalid fixtures, {jcs_golden_count} JCS byte goldens, "
            f"{machine_expectation_count} machine expectations in {machine_matrix_count} matrices, "
            f"and config JCS hash {config_digest}"
        )
        return 0
    except Exception as error:  # one concise failure path for automation and local use
        failures.append(str(error))

    for failure in failures:
        print(f"error: {failure}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
