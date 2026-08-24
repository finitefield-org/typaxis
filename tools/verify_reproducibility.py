#!/usr/bin/env python3
"""Verify checkout-name-independent blank PDFs and release ZIP bytes."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from typing import Mapping, Sequence

import release


class ReproducibilityError(Exception):
    pass


@dataclass(frozen=True)
class ReproducibilityResult:
    tree: str
    pdf_sha256: str
    release_sha256: str


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
    return parser.parse_args(arguments)


def main(arguments: list[str] | None = None) -> int:
    options = _parse_arguments(sys.argv[1:] if arguments is None else arguments)
    try:
        result = verify_reproducibility(
            options.repository, revision=options.revision, cargo=options.cargo
        )
    except (ReproducibilityError, release.ReleaseError, OSError, UnicodeError) as error:
        print(f"reproducibility error: {error}", file=sys.stderr)
        return 1
    print(f"tree {result.tree}")
    print(f"blank-pdf sha256 {result.pdf_sha256}")
    print(f"release-zip sha256 {result.release_sha256}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
