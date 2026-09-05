#!/usr/bin/env python3
"""Verify checkout-name-independent blank PDFs and release ZIP bytes."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from typing import Mapping, Sequence

import release


class ReproducibilityError(Exception):
    pass


MACHINE_TARGET_DIRECTORY_NAMES = ("target-clean", "target-alpha", "target-bravo")
MACHINE_TEMPORARY_PREFIX = "typaxis-machine-gate-"


@dataclass(frozen=True)
class ReproducibilityResult:
    tree: str
    pdf_sha256: str
    release_sha256: str


@dataclass(frozen=True)
class MachineReproducibilityResult:
    revision: str
    source_snapshot_sha256: str
    binary_version: str
    binary_sha256: str
    artifact_sha256: dict[str, str]

    def as_json(self) -> dict[str, object]:
        return {
            "artifacts": dict(sorted(self.artifact_sha256.items())),
            "binary_sha256": self.binary_sha256,
            "binary_version": self.binary_version,
            "revision": self.revision,
            "source_snapshot_sha256": self.source_snapshot_sha256,
        }


@dataclass(frozen=True)
class PrivateStagingReproducibilityResult:
    revision: str
    source_snapshot_sha256: str
    published_artifact_set_sha256: str
    artifact_sha256: dict[str, str]

    def as_json(self) -> dict[str, object]:
        return {
            "artifacts": dict(sorted(self.artifact_sha256.items())),
            "published_artifact_set_sha256": self.published_artifact_set_sha256,
            "revision": self.revision,
            "source_snapshot_sha256": self.source_snapshot_sha256,
        }


def filtered_environment(base: Mapping[str, str] | None = None) -> dict[str, str]:
    """Remove all ambient Typaxis config before build and CLI execution."""

    source = os.environ if base is None else base
    environment = {
        key: value
        for key, value in source.items()
        if not key.upper().startswith("TYPAXIS_")
    }
    # These do not alter Typaxis semantics, but keep tool output and build behavior stable.
    environment.update(
        {
            "CARGO_INCREMENTAL": "0",
            "LC_ALL": "C",
            "LANG": "C",
            "TZ": "UTC",
        }
    )
    environment.pop("CARGO_TARGET_DIR", None)
    environment.pop("RUSTFLAGS", None)
    environment.pop("RUSTDOCFLAGS", None)
    environment.pop("CARGO_ENCODED_RUSTFLAGS", None)
    return environment


def _run_checked(
    command: Sequence[os.PathLike[str] | str],
    *,
    cwd: Path | None,
    environment: Mapping[str, str],
) -> None:
    rendered = [os.fspath(argument) for argument in command]
    try:
        completed = subprocess.run(
            rendered,
            cwd=None if cwd is None else os.fspath(cwd),
            env=dict(environment),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise ReproducibilityError(
            f"cannot execute {rendered[0]!r}: {error}"
        ) from error
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", "replace").strip()
        stdout = completed.stdout.decode("utf-8", "replace").strip()
        detail = stderr or stdout or f"exit status {completed.returncode}"
        raise ReproducibilityError(f"{' '.join(rendered)} failed: {detail}")


def _run_capture(
    command: Sequence[os.PathLike[str] | str],
    *,
    cwd: Path | None,
    environment: Mapping[str, str],
) -> bytes:
    rendered = [os.fspath(argument) for argument in command]
    try:
        completed = subprocess.run(
            rendered,
            cwd=None if cwd is None else os.fspath(cwd),
            env=dict(environment),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise ReproducibilityError(
            f"cannot execute {rendered[0]!r}: {error}"
        ) from error
    if completed.returncode != 0:
        stderr = completed.stderr.decode("utf-8", "replace").strip()
        stdout = completed.stdout.decode("utf-8", "replace").strip()
        detail = stderr or stdout or f"exit status {completed.returncode}"
        raise ReproducibilityError(f"{' '.join(rendered)} failed: {detail}")
    return completed.stdout


def _materialize_checkout(
    repository: Path,
    destination: Path,
    tree: str,
    environment: Mapping[str, str],
) -> None:
    git_environment = release.git_environment(environment)
    _run_checked(
        [
            "git",
            "clone",
            "--quiet",
            "--no-checkout",
            "--no-local",
            "--",
            repository,
            destination,
        ],
        cwd=None,
        environment=git_environment,
    )
    # A tree ID, rather than a moving branch name, owns every copied source byte.
    _run_checked(
        ["git", "-C", destination, "cat-file", "-e", f"{tree}^{{tree}}"],
        cwd=None,
        environment=git_environment,
    )
    _run_checked(
        ["git", "-C", destination, "read-tree", "--reset", tree],
        cwd=None,
        environment=git_environment,
    )
    _run_checked(
        ["git", "-C", destination, "checkout-index", "--all", "--force"],
        cwd=None,
        environment=git_environment,
    )


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _exact_files_equal(first: Path, second: Path) -> bool:
    if first.stat().st_size != second.stat().st_size:
        return False
    with first.open("rb") as left, second.open("rb") as right:
        while True:
            left_chunk = left.read(1024 * 1024)
            right_chunk = right.read(1024 * 1024)
            if left_chunk != right_chunk:
                return False
            if not left_chunk:
                return True


def _build_blank_pdf(
    checkout: Path,
    output: Path,
    *,
    cargo: str,
    environment: Mapping[str, str],
) -> bytes:
    manifest = checkout / "workspace/Cargo.toml"
    blank_input = checkout / "samples/minimal/empty.tsf"
    if not manifest.is_file() or not blank_input.is_file():
        raise ReproducibilityError(
            "selected Git tree lacks workspace/Cargo.toml or samples/minimal/empty.tsf"
        )
    build_environment = dict(environment)
    target = checkout / "workspace/target"
    build_environment["CARGO_TARGET_DIR"] = os.fspath(target)
    _run_checked(
        [
            cargo,
            "build",
            "--manifest-path",
            manifest,
            "--locked",
            "--package",
            "typaxis-cli",
            "--bin",
            "typaxis",
        ],
        cwd=checkout,
        environment=build_environment,
    )
    executable = target / "debug" / ("typaxis.exe" if os.name == "nt" else "typaxis")
    if not executable.is_file():
        raise ReproducibilityError(f"Cargo did not produce {executable}")
    _run_checked(
        [
            executable,
            "build",
            blank_input,
            "-o",
            output,
            "--no-compress",
        ],
        cwd=checkout,
        environment=environment,
    )
    if not output.is_file():
        raise ReproducibilityError(f"Typaxis did not publish {output}")
    return output.read_bytes()


def _resolve_commit(
    repository: Path,
    revision: str,
    environment: Mapping[str, str],
) -> str:
    payload = _run_capture(
        ["git", "-C", repository, "rev-parse", "--verify", f"{revision}^{{commit}}"],
        cwd=None,
        environment=release.git_environment(environment),
    )
    try:
        commit = payload.decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise ReproducibilityError("Git revision is not ASCII") from error
    if len(commit) not in {40, 64} or any(
        character not in "0123456789abcdef" for character in commit
    ):
        raise ReproducibilityError(f"Git returned an invalid commit ID: {commit!r}")
    return commit


def _listed_worktree_files(
    repository: Path,
    environment: Mapping[str, str],
) -> list[Path]:
    payload = _run_capture(
        [
            "git",
            "-C",
            repository,
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        cwd=None,
        environment=release.git_environment(environment),
    )
    files: list[Path] = []
    for raw in payload.split(b"\0"):
        if not raw:
            continue
        relative = Path(os.fsdecode(raw))
        if relative.is_absolute() or ".." in relative.parts:
            raise ReproducibilityError(f"Git listed an unsafe source path: {relative}")
        source = repository / relative
        if source.is_symlink() or source.is_file():
            files.append(relative)
        elif source.exists():
            raise ReproducibilityError(f"Git source entry is not a file: {relative}")
        # A tracked deletion is intentionally absent from the worktree snapshot.
    files.sort(key=lambda path: os.fsencode(path))
    if not files:
        raise ReproducibilityError("the current worktree source snapshot is empty")
    return files


def _source_snapshot_sha256(repository: Path, files: Sequence[Path]) -> str:
    digest = hashlib.sha256()
    for relative in files:
        source = repository / relative
        encoded_path = os.fsencode(relative)
        if source.is_symlink():
            kind = b"symlink"
            payload = os.fsencode(os.readlink(source))
            executable = False
        elif source.is_file():
            kind = b"file"
            payload = source.read_bytes()
            executable = bool(source.stat().st_mode & 0o111)
        else:
            raise ReproducibilityError(f"source changed during snapshot: {relative}")
        digest.update(len(encoded_path).to_bytes(8, "big"))
        digest.update(encoded_path)
        digest.update(kind)
        digest.update(b"x" if executable else b"-")
        digest.update(len(payload).to_bytes(8, "big"))
        digest.update(payload)
    return digest.hexdigest()


def _materialize_worktree_snapshot(
    repository: Path,
    destination: Path,
    files: Sequence[Path],
    *,
    reverse_creation_order: bool = False,
) -> None:
    destination.mkdir()
    ordered_files = reversed(files) if reverse_creation_order else iter(files)
    for relative in ordered_files:
        source = repository / relative
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        if source.is_symlink():
            target.symlink_to(os.readlink(source))
        elif source.is_file():
            shutil.copy2(source, target)
        else:
            raise ReproducibilityError(f"source changed during copy: {relative}")


def _machine_fixture_arguments(
    expected: dict[str, object],
    output: Path,
) -> list[str]:
    command = expected.get("command")
    arguments = expected.get("arguments")
    if command != "build-package" or not isinstance(arguments, list) or not all(
        isinstance(argument, str) for argument in arguments
    ):
        raise ReproducibilityError("machine fixture does not define build-package arguments")
    rendered = [
        argument.replace("$OUTPUT", os.fspath(output))
        for argument in arguments
    ]
    if "--trace" not in rendered:
        rendered.extend(["--trace", os.fspath(output / "trace.json")])
    if "--trace-text" not in rendered:
        rendered.append("--trace-text")
    if "--no-compress" not in rendered:
        rendered.append("--no-compress")
    return [command, *rendered]


def _build_machine_binary(
    checkout: Path,
    target: Path,
    *,
    cargo: str,
    environment: Mapping[str, str],
) -> Path:
    manifest = checkout / "workspace/Cargo.toml"
    if not manifest.is_file():
        raise ReproducibilityError("source snapshot lacks workspace/Cargo.toml")
    build_environment = dict(environment)
    build_environment["CARGO_TARGET_DIR"] = os.fspath(target)
    build_environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(
        _machine_build_rustflags(checkout, target)
    )
    _run_checked(
        [
            cargo,
            "build",
            "--manifest-path",
            manifest,
            "--locked",
            "--package",
            "typaxis-cli",
            "--bin",
            "typaxis",
        ],
        cwd=checkout,
        environment=build_environment,
    )
    executable = target / "debug" / ("typaxis.exe" if os.name == "nt" else "typaxis")
    if not executable.is_file():
        raise ReproducibilityError(f"Cargo did not produce {executable}")
    return executable


def _machine_build_rustflags(checkout: Path, target: Path) -> tuple[str, ...]:
    """Keep release-gate binaries independent of source and target paths.

    macOS linkers retain native archive member paths in the final symbol table.
    Remapping Rust paths alone therefore leaves checkout-specific bytes from
    crates with native objects (notably ``psm``).  The evidence binary does not
    need debug or local symbols, so strip them during the reproducibility build.
    """

    return (
        f"--remap-path-prefix={os.fspath(checkout)}=/typaxis-source",
        f"--remap-path-prefix={os.fspath(target)}=/typaxis-target",
        "-C",
        "debuginfo=0",
        "-C",
        "strip=symbols",
    )


def _run_machine_fixture(
    checkout: Path,
    fixture_relative: Path,
    executable: Path,
    output: Path,
    environment: Mapping[str, str],
) -> tuple[str, dict[str, bytes]]:
    expected_path = checkout / fixture_relative
    try:
        expected = json.loads(expected_path.read_bytes())
    except (OSError, json.JSONDecodeError, UnicodeError) as error:
        raise ReproducibilityError(f"cannot load machine fixture {expected_path}: {error}") from error
    if (
        not isinstance(expected, dict)
        or expected.get("fixture_class") != "positive"
        or not isinstance(expected.get("expected"), dict)
        or expected["expected"].get("exit_code") != 0
    ):
        raise ReproducibilityError("machine reproducibility requires a positive expected.json")
    output.mkdir()
    fixture_directory = expected_path.parent
    _run_checked(
        [executable, *_machine_fixture_arguments(expected, output)],
        cwd=fixture_directory,
        environment=environment,
    )
    capabilities = _run_capture(
        [executable, "capabilities", "--format", "json"],
        cwd=fixture_directory,
        environment=environment,
    )
    artifacts = {
        "capabilities": capabilities,
        "diagnostics": (output / "diagnostics.json").read_bytes(),
        "manifest": (output / "manifest.json").read_bytes(),
        "pdf": (output / "output.pdf").read_bytes(),
        "trace": (output / "trace.json").read_bytes(),
    }
    version = _run_capture(
        [executable, "--version"], cwd=fixture_directory, environment=environment
    ).decode("utf-8", "strict").strip()
    if not version.startswith("typaxis "):
        raise ReproducibilityError(f"machine binary returned an invalid version: {version!r}")
    return version, artifacts


def verify_machine_reproducibility(
    repository: Path,
    machine_fixture: Path,
    *,
    revision: str = "HEAD",
    cargo: str = "cargo",
) -> MachineReproducibilityResult:
    root = release.repository_root(repository)
    environment = filtered_environment()
    selected_revision = _resolve_commit(root, revision, environment)
    head_revision = _resolve_commit(root, "HEAD", environment)
    if selected_revision != head_revision:
        raise ReproducibilityError(
            "machine worktree mode requires --revision to resolve to the current HEAD"
        )
    expected_path = machine_fixture
    if not expected_path.is_absolute():
        expected_path = root / expected_path
    expected_path = expected_path.resolve(strict=True)
    try:
        fixture_relative = expected_path.relative_to(root)
    except ValueError as error:
        raise ReproducibilityError("machine fixture must be inside the repository") from error

    files = _listed_worktree_files(root, environment)
    source_snapshot_sha256 = _source_snapshot_sha256(root, files)
    with tempfile.TemporaryDirectory(prefix=MACHINE_TEMPORARY_PREFIX) as raw_temporary:
        temporary = Path(raw_temporary)
        first_checkout = temporary / "checkout-alpha"
        second_checkout = temporary / "typaxis-source-under-a-different-name"
        _materialize_worktree_snapshot(root, first_checkout, files)
        _materialize_worktree_snapshot(root, second_checkout, files)
        if (
            _source_snapshot_sha256(first_checkout, files) != source_snapshot_sha256
            or _source_snapshot_sha256(second_checkout, files) != source_snapshot_sha256
        ):
            raise ReproducibilityError("aliased source snapshots differ from the worktree")

        first_binary = _build_machine_binary(
            first_checkout,
            temporary / MACHINE_TARGET_DIRECTORY_NAMES[1],
            cargo=cargo,
            environment=environment,
        )
        second_binary = _build_machine_binary(
            second_checkout,
            # Darwin's linker lays out native archive paths before stripping
            # local symbols.  Equal-length isolated target names keep that
            # non-source input from obscuring the checkout-name assertion.
            temporary / MACHINE_TARGET_DIRECTORY_NAMES[2],
            cargo=cargo,
            environment=environment,
        )
        first_binary_bytes = first_binary.read_bytes()
        second_binary_bytes = second_binary.read_bytes()
        if first_binary_bytes != second_binary_bytes:
            raise ReproducibilityError(
                "machine binary bytes differ across checkout names: "
                f"{_sha256(first_binary_bytes)} != {_sha256(second_binary_bytes)}"
            )

        first_version, first_artifacts = _run_machine_fixture(
            first_checkout,
            fixture_relative,
            first_binary,
            temporary / "outputs-alpha",
            environment,
        )
        second_version, second_artifacts = _run_machine_fixture(
            second_checkout,
            fixture_relative,
            second_binary,
            temporary / "outputs-beta",
            environment,
        )
        if first_version != second_version:
            raise ReproducibilityError("machine binary versions differ across checkout names")
        if first_artifacts.keys() != second_artifacts.keys():
            raise ReproducibilityError("machine artifact sets differ across checkout names")
        for name in first_artifacts:
            if first_artifacts[name] != second_artifacts[name]:
                raise ReproducibilityError(
                    f"machine {name} bytes differ across checkout names: "
                    f"{_sha256(first_artifacts[name])} != {_sha256(second_artifacts[name])}"
                )
        return MachineReproducibilityResult(
            revision=selected_revision,
            source_snapshot_sha256=source_snapshot_sha256,
            binary_version=first_version,
            binary_sha256=_sha256(first_binary_bytes),
            artifact_sha256={
                name: _sha256(payload) for name, payload in sorted(first_artifacts.items())
            },
        )


_PRIVATE_PRECOMPOSED_VECTOR_ARTIFACTS = (
    "artifact-index.json",
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
    "tagged-pdf-manifest.json",
    "verification.json",
)


def _private_staging_artifacts(checkout: Path) -> dict[str, bytes]:
    directory = checkout / "workspace/target/machine-e2e/precomposed-vector"
    if not directory.is_dir():
        raise ReproducibilityError(
            f"private precomposed-vector test did not publish {directory}"
        )
    names = tuple(
        sorted(
            path.relative_to(directory).as_posix()
            for path in directory.rglob("*")
            if path.is_file()
        )
    )
    if names != _PRIVATE_PRECOMPOSED_VECTOR_ARTIFACTS:
        missing = sorted(set(_PRIVATE_PRECOMPOSED_VECTOR_ARTIFACTS) - set(names))
        extra = sorted(set(names) - set(_PRIVATE_PRECOMPOSED_VECTOR_ARTIFACTS))
        raise ReproducibilityError(
            "private precomposed-vector artifact set differs: "
            f"missing={missing}, extra={extra}"
        )
    return {name: (directory / name).read_bytes() for name in names}


def _run_private_precomposed_vector_test(
    checkout: Path,
    target: Path,
    *,
    cargo: str,
    environment: Mapping[str, str],
) -> dict[str, bytes]:
    manifest = checkout / "workspace/Cargo.toml"
    build_environment = dict(environment)
    build_environment["CARGO_TARGET_DIR"] = os.fspath(target)
    build_environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(
        _machine_build_rustflags(checkout, target)
    )
    _run_checked(
        [
            cargo,
            "test",
            "--manifest-path",
            manifest,
            "--locked",
            "--package",
            "typaxis-cli",
            "machine_precomposed_vector_closes_production_pipeline",
            "--",
            "--test-threads=1",
        ],
        cwd=checkout,
        environment=build_environment,
    )
    artifacts = _private_staging_artifacts(checkout)
    _run_checked(
        [
            sys.executable,
            checkout / "tools/verify_precomposed_vector.py",
            checkout / "workspace/target/machine-e2e/precomposed-vector",
            "--repository",
            checkout,
        ],
        cwd=checkout,
        environment=environment,
    )
    return artifacts


def verify_private_staging_reproducibility(
    repository: Path,
    *,
    revision: str = "HEAD",
    cargo: str = "cargo",
) -> PrivateStagingReproducibilityResult:
    """Verify MI4-V18 bytes across path, locale, timezone, and creation order."""

    root = release.repository_root(repository)
    environment = filtered_environment()
    selected_revision = _resolve_commit(root, revision, environment)
    head_revision = _resolve_commit(root, "HEAD", environment)
    if selected_revision != head_revision:
        raise ReproducibilityError(
            "private staging mode requires --revision to resolve to the current HEAD"
        )
    files = _listed_worktree_files(root, environment)
    source_snapshot_sha256 = _source_snapshot_sha256(root, files)
    with tempfile.TemporaryDirectory(prefix=MACHINE_TEMPORARY_PREFIX) as raw_temporary:
        temporary = Path(raw_temporary)
        first_checkout = temporary / "checkout-alpha"
        second_checkout = temporary / "typaxis-source-under-a-different-name"
        _materialize_worktree_snapshot(root, first_checkout, files)
        _materialize_worktree_snapshot(
            root,
            second_checkout,
            files,
            reverse_creation_order=True,
        )
        if (
            _source_snapshot_sha256(first_checkout, files) != source_snapshot_sha256
            or _source_snapshot_sha256(second_checkout, files)
            != source_snapshot_sha256
        ):
            raise ReproducibilityError("aliased source snapshots differ from the worktree")

        first_environment = dict(environment)
        first_environment.update({"LC_ALL": "C", "LANG": "C", "TZ": "UTC"})
        second_environment = dict(environment)
        second_environment.update(
            {
                "LC_ALL": "ja_JP.UTF-8",
                "LANG": "ja_JP.UTF-8",
                "TZ": "Asia/Tokyo",
            }
        )
        first_artifacts = _run_private_precomposed_vector_test(
            first_checkout,
            temporary / MACHINE_TARGET_DIRECTORY_NAMES[1],
            cargo=cargo,
            environment=first_environment,
        )
        second_artifacts = _run_private_precomposed_vector_test(
            second_checkout,
            temporary / MACHINE_TARGET_DIRECTORY_NAMES[2],
            cargo=cargo,
            environment=second_environment,
        )
        for name in _PRIVATE_PRECOMPOSED_VECTOR_ARTIFACTS:
            if first_artifacts[name] != second_artifacts[name]:
                raise ReproducibilityError(
                    f"private staging {name} bytes differ across environments: "
                    f"{_sha256(first_artifacts[name])} != "
                    f"{_sha256(second_artifacts[name])}"
                )
        digest = hashlib.sha256()
        for name in _PRIVATE_PRECOMPOSED_VECTOR_ARTIFACTS:
            encoded_name = name.encode("utf-8")
            payload = first_artifacts[name]
            digest.update(len(encoded_name).to_bytes(8, "big"))
            digest.update(encoded_name)
            digest.update(len(payload).to_bytes(8, "big"))
            digest.update(payload)
        return PrivateStagingReproducibilityResult(
            revision=selected_revision,
            source_snapshot_sha256=source_snapshot_sha256,
            published_artifact_set_sha256=digest.hexdigest(),
            artifact_sha256={
                name: _sha256(payload)
                for name, payload in sorted(first_artifacts.items())
            },
        )


def verify_reproducibility(
    repository: Path,
    *,
    revision: str = "HEAD",
    cargo: str = "cargo",
) -> ReproducibilityResult:
    root = release.repository_root(repository)
    tree = release.resolve_tree(root, revision)
    environment = filtered_environment()
    with tempfile.TemporaryDirectory(prefix="typaxis-repro-") as raw_temporary:
        temporary = Path(raw_temporary)
        first_checkout = temporary / "checkout-alpha"
        second_checkout = temporary / "typaxis-source-under-a-different-name"
        _materialize_checkout(root, first_checkout, tree, environment)
        _materialize_checkout(root, second_checkout, tree, environment)

        outputs = temporary / "outputs"
        outputs.mkdir()
        first_pdf_path = outputs / "first.pdf"
        second_pdf_path = outputs / "second.pdf"
        first_pdf = _build_blank_pdf(
            first_checkout,
            first_pdf_path,
            cargo=cargo,
            environment=environment,
        )
        second_pdf = _build_blank_pdf(
            second_checkout,
            second_pdf_path,
            cargo=cargo,
            environment=environment,
        )
        if first_pdf != second_pdf:
            raise ReproducibilityError(
                "blank PDF bytes differ across checkout names: "
                f"{_sha256(first_pdf)} != {_sha256(second_pdf)}"
            )

        first_zip = outputs / "first.zip"
        second_zip = outputs / "second.zip"
        first_digest = release.build_release(
            first_checkout, first_zip, revision=tree
        )
        second_digest = release.build_release(
            second_checkout, second_zip, revision=tree
        )
        first_verified = release.verify_release_archive(
            first_checkout, first_zip, revision=tree
        )
        second_verified = release.verify_release_archive(
            second_checkout, second_zip, revision=tree
        )
        if (
            first_digest != first_verified
            or second_digest != second_verified
            or first_digest != second_digest
            or not _exact_files_equal(first_zip, second_zip)
        ):
            raise ReproducibilityError(
                "release ZIP bytes differ across checkout names: "
                f"{first_verified} != {second_verified}"
            )
        # Both Cargo target trees existed before release generation. Exact Git-tree
        # verification above proves that neither entered either archive.
        if not (first_checkout / "workspace/target").is_dir() or not (
            second_checkout / "workspace/target"
        ).is_dir():
            raise ReproducibilityError("independent Cargo target directories were not created")

        return ReproducibilityResult(
            tree=tree,
            pdf_sha256=_sha256(first_pdf),
            release_sha256=first_digest,
        )


def _parse_arguments(arguments: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--revision", default="HEAD", help="Git tree-ish to verify")
    parser.add_argument("--cargo", default="cargo", help="Cargo executable")
    parser.add_argument(
        "--machine-fixture",
        type=Path,
        help="positive machine expected.json to verify across aliased worktree snapshots",
    )
    parser.add_argument(
        "--private-staging-test",
        choices=("precomposed-vector",),
        help="private staging integration gate to verify across hostile environments",
    )
    return parser.parse_args(arguments)


def main(arguments: list[str] | None = None) -> int:
    options = _parse_arguments(sys.argv[1:] if arguments is None else arguments)
    if options.machine_fixture is not None and options.private_staging_test is not None:
        print(
            "reproducibility error: --machine-fixture and --private-staging-test "
            "are mutually exclusive",
            file=sys.stderr,
        )
        return 2
    try:
        if options.private_staging_test is not None:
            private_staging = verify_private_staging_reproducibility(
                options.repository,
                revision=options.revision,
                cargo=options.cargo,
            )
        elif options.machine_fixture is not None:
            machine = verify_machine_reproducibility(
                options.repository,
                options.machine_fixture,
                revision=options.revision,
                cargo=options.cargo,
            )
        else:
            result = verify_reproducibility(
                options.repository, revision=options.revision, cargo=options.cargo
            )
    except (ReproducibilityError, release.ReleaseError, OSError, UnicodeError) as error:
        print(f"reproducibility error: {error}", file=sys.stderr)
        return 1
    if options.private_staging_test is not None:
        print(
            json.dumps(
                private_staging.as_json(),
                ensure_ascii=False,
                separators=(",", ":"),
                sort_keys=True,
            )
        )
        return 0
    if options.machine_fixture is not None:
        print(
            json.dumps(
                machine.as_json(), ensure_ascii=False, separators=(",", ":"), sort_keys=True
            )
        )
        return 0
    print(f"tree {result.tree}")
    print(f"blank-pdf sha256 {result.pdf_sha256}")
    print(f"release-zip sha256 {result.release_sha256}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
