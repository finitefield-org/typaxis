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
REQUIRED_TOOLS = {"cargo", "mutool", "pdfinfo", "pdftotext", "python", "rustc"}
PUBLIC_PROFILES = {
    "typaxis.machine-pdf/basic-document-1",
    "typaxis.machine-pdf/paragraph-1",
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
    ]:
        raise MachineProfileError("capabilities changed the accepted contract migration set")
    if capabilities.get("contract") != "typaxis.contract/1.2":
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
    if profile == "typaxis.machine-pdf/basic-document-1":
        if not isinstance(manifest_flow, str) or len(manifest_flow) != 64:
            raise MachineProfileError("basic-document-1 lacks its selected flow registry binding")
    elif manifest_flow is not None:
        raise MachineProfileError("paragraph-1 unexpectedly carries a basic flow registry")


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
    return FixtureResult(
        expected_path=expected_path,
        expected=expected,
        run_directories=tuple(run_directories),
        artifacts=baseline,
        differential=differential,
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
        if {check["name"] for check in checks} != REQUIRED_CHECKS:
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
        if {check["name"] for check in evidence["checks"]} != REQUIRED_CHECKS:
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
