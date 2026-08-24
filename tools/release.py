#!/usr/bin/env python3
"""Build and verify the stored, byte-reproducible Typaxis release ZIP."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path, PurePosixPath
import re
import secrets
import stat
import subprocess
import sys
import tempfile
import unicodedata
from typing import Mapping
import zipfile


VERSION = "0.1.0"
# This is deliberately a source constant, not a checkout-directory basename.
ARCHIVE_ROOT = f"typaxis-{VERSION}"
ZIP_TIMESTAMP = (1980, 1, 1, 0, 0, 0)
ZIP_CREATE_VERSION = 20
ZIP_EXTRACT_VERSION = 10
ZIP_UTF8_FLAG = 0x800
CANONICAL_MODE = stat.S_IFREG | 0o644
PUBLISHED_FILE_MODE = 0o644

_HEX_OBJECT_ID = re.compile(r"(?:[0-9a-f]{40}|[0-9a-f]{64})\Z")
_WINDOWS_FORBIDDEN_CHARACTERS = frozenset('<>:"|?*')
_WINDOWS_RESERVED_NAMES = frozenset(
    {"con", "prn", "aux", "nul"}
    | {f"com{index}" for index in range(1, 10)}
    | {f"lpt{index}" for index in range(1, 10)}
    | {f"com{index}" for index in "¹²³"}
    | {f"lpt{index}" for index in "¹²³"}
)
_FORBIDDEN_RELEASE_COMPONENTS = frozenset({".git", "__pycache__", "target"})


class ReleaseError(Exception):
    """A release failed before it could be published."""


class ReleaseDurabilityError(ReleaseError):
    """The archive is visible, but its parent-directory sync failed."""

    def __init__(self, output: Path, digest: str, source: OSError) -> None:
        self.output = output
        self.digest = digest
        self.source = source
        super().__init__(
            f"release is already visible at {output} (sha256 {digest}), "
            f"but parent-directory synchronization failed: {source}"
        )


def git_environment(base: Mapping[str, str] | None = None) -> dict[str, str]:
    """Return an environment in which Git cannot be redirected by ambient state."""

    source = os.environ if base is None else base
    environment = {
        key: value for key, value in source.items() if not key.upper().startswith("GIT_")
    }
    environment.update(
        {
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_NO_REPLACE_OBJECTS": "1",
            "GIT_OPTIONAL_LOCKS": "0",
            "LC_ALL": "C",
            "LANG": "C",
            "TZ": "UTC",
        }
    )
    return environment


def _git(repository: Path, *arguments: str) -> bytes:
    try:
        completed = subprocess.run(
            ["git", "-C", os.fspath(repository), *arguments],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=git_environment(),
            check=False,
        )
    except OSError as error:
        raise ReleaseError(f"cannot execute git: {error}") from error
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise ReleaseError(f"git {' '.join(arguments)} failed: {detail}")
    return completed.stdout


def _single_git_line(raw: bytes, description: str) -> bytes:
    if not raw.endswith(b"\n") or b"\n" in raw[:-1] or b"\0" in raw:
        raise ReleaseError(f"git returned a noncanonical {description}")
    return raw[:-1]


def repository_root(repository: Path) -> Path:
    raw = _git(Path(repository), "rev-parse", "--show-toplevel")
    root = Path(os.fsdecode(_single_git_line(raw, "repository path")))
    if not root.is_dir():
        raise ReleaseError(f"Git repository root is not a directory: {root}")
    return root


def resolve_tree(repository: Path, revision: str = "HEAD") -> str:
    """Resolve a user tree-ish once, preventing ref races during packaging."""

    if not revision or "\0" in revision or "\n" in revision or "\r" in revision:
        raise ReleaseError("revision must be one nonempty line")
    raw = _git(
        repository_root(repository),
        "rev-parse",
        "--verify",
        "--end-of-options",
        f"{revision}^{{tree}}",
    )
    try:
        object_id = _single_git_line(raw, "tree object ID").decode("ascii", "strict")
    except UnicodeDecodeError as error:
        raise ReleaseError("git returned a non-ASCII tree object ID") from error
    if _HEX_OBJECT_ID.fullmatch(object_id) is None:
        raise ReleaseError("git returned an invalid tree object ID")
    return object_id


def _portable_component(component: str, path: str) -> None:
    folded = component.casefold()
    if folded in _FORBIDDEN_RELEASE_COMPONENTS:
        raise ReleaseError(f"build or repository metadata is not releasable: {path!r}")
    if component[-1] in {" ", "."}:
        raise ReleaseError(f"archive path has a nonportable trailing character: {path!r}")
    if any(character in _WINDOWS_FORBIDDEN_CHARACTERS for character in component):
        raise ReleaseError(f"archive path is not portable across supported extractors: {path!r}")
    # Win32 strips spaces and dots before recognizing DOS device names, including
    # before an extension (for example, ``CON .txt``).
    device_name = component.split(".", 1)[0].rstrip(" .").casefold()
    if device_name in _WINDOWS_RESERVED_NAMES:
        raise ReleaseError(f"archive path uses a reserved device name: {path!r}")


def _validated_member_path(path: str) -> str:
    if (
        not path
        or "\\" in path
        or "\0" in path
        or any(ord(character) < 0x20 or ord(character) == 0x7F for character in path)
    ):
        raise ReleaseError(f"unsafe archive path {path!r}")
    components = path.split("/")
    if any(component in {"", ".", ".."} for component in components):
        raise ReleaseError(f"unsafe archive path {path!r}")
    for component in components:
        _portable_component(component, path)
    parsed = PurePosixPath(path)
    if parsed.is_absolute():
        raise ReleaseError(f"unsafe archive path {path!r}")
    member = f"{ARCHIVE_ROOT}/{path}"
    if not member.startswith(f"{ARCHIVE_ROOT}/"):
        raise ReleaseError(f"archive path escaped its fixed root: {path!r}")
    return member


def _portable_collision_key(path: str) -> str:
    return "/".join(
        unicodedata.normalize("NFC", component).casefold()
        for component in path.split("/")
    )


def _register_portable_node(
    nodes: dict[str, tuple[str, str]], path: str, kind: str
) -> None:
    key = _portable_collision_key(path)
    previous = nodes.get(key)
    if previous is not None and previous != (path, kind):
        previous_path, previous_kind = previous
        raise ReleaseError(
            "portable extraction collision between "
            f"{previous_kind} {previous_path!r} and {kind} {path!r}"
        )
    nodes[key] = (path, kind)


def _tree_entries(repository: Path, revision: str) -> list[tuple[str, str]]:
    root = repository_root(repository)
    tree = resolve_tree(root, revision)
    raw = _git(root, "ls-tree", "-r", "-z", "--full-tree", tree)
    entries: list[tuple[str, str]] = []
    exact_paths: set[str] = set()
    portable_nodes: dict[str, tuple[str, str]] = {}
    for record in raw.split(b"\0"):
        if not record:
            continue
        try:
            header, raw_path = record.split(b"\t", 1)
            mode, object_kind, raw_oid = header.split(b" ", 2)
            path = raw_path.decode("utf-8", "strict")
            object_id = raw_oid.decode("ascii", "strict")
        except (UnicodeDecodeError, ValueError) as error:
            raise ReleaseError(
                "git tree contains a noncanonical record or non-UTF-8 path"
            ) from error
        if object_kind != b"blob" or mode not in {b"100644", b"100755"}:
            raise ReleaseError(f"release tree entry is not a regular file: {path!r}")
        if _HEX_OBJECT_ID.fullmatch(object_id) is None:
            raise ReleaseError(f"release tree entry has an invalid object ID: {path!r}")
        _validated_member_path(path)
        if path in exact_paths:
            raise ReleaseError(f"release tree contains duplicate path {path!r}")
        exact_paths.add(path)
        components = path.split("/")
        for end in range(1, len(components)):
            _register_portable_node(
                portable_nodes, "/".join(components[:end]), "directory"
            )
        _register_portable_node(portable_nodes, path, "file")
        entries.append((path, object_id))
    if not entries:
        raise ReleaseError("release tree contains no regular files")
    entries.sort(key=lambda entry: entry[0].encode("utf-8"))
    return entries


def _zip_info(member: str) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(member, ZIP_TIMESTAMP)
    info.compress_type = zipfile.ZIP_STORED
    info.create_system = 3
    info.create_version = ZIP_CREATE_VERSION
    info.extract_version = ZIP_EXTRACT_VERSION
    info.external_attr = CANONICAL_MODE << 16
    info.internal_attr = 0
    info.extra = b""
    info.comment = b""
    return info


def _write_archive(repository: Path, revision: str, output) -> str:
    entries = _tree_entries(repository, revision)
    if len(entries) >= zipfile.ZIP_FILECOUNT_LIMIT:
        raise ReleaseError("release tree has too many entries for a non-Zip64 archive")
    digest = hashlib.sha256()
    output.seek(0)
    output.truncate()
    with zipfile.ZipFile(
        output,
        mode="w",
        compression=zipfile.ZIP_STORED,
        allowZip64=False,
        strict_timestamps=True,
    ) as archive:
        archive.comment = b""
        for path, object_id in entries:
            payload = _git(repository, "cat-file", "blob", object_id)
            if len(payload) >= zipfile.ZIP64_LIMIT:
                raise ReleaseError(f"release entry is too large for non-Zip64 output: {path!r}")
            archive.writestr(
                _zip_info(_validated_member_path(path)),
                payload,
                compress_type=zipfile.ZIP_STORED,
            )
    os.fchmod(output.fileno(), PUBLISHED_FILE_MODE)
    output.flush()
    os.fsync(output.fileno())
    output.seek(0)
    while True:
        chunk = output.read(1024 * 1024)
        if not chunk:
            break
        digest.update(chunk)
    return digest.hexdigest()


def _open_parent_directory(directory: Path) -> int:
    if os.name != "posix":
        raise ReleaseError(
            "atomic release publication requires POSIX directory-relative primitives"
        )
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_CLOEXEC", 0)
    flags |= getattr(os, "O_NOFOLLOW", 0)
    return os.open(directory, flags)


def _create_temporary(parent_descriptor: int) -> tuple[str, int]:
    flags = os.O_RDWR | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
    flags |= getattr(os, "O_CLOEXEC", 0)
    for _ in range(128):
        name = f".typaxis-release-{secrets.token_hex(12)}.tmp"
        try:
            return name, os.open(name, flags, 0o600, dir_fd=parent_descriptor)
        except FileExistsError:
            continue
    raise ReleaseError("could not allocate a unique release temporary file")


def _sync_directory_descriptor(descriptor: int) -> None:
    os.fsync(descriptor)


def build_release(
    repository: Path,
    output: Path,
    *,
    revision: str = "HEAD",
    force: bool = False,
) -> str:
    root = repository_root(repository)
    tree = resolve_tree(root, revision)
    output = Path(os.path.abspath(os.fspath(output)))
    if not output.name or output.name in {".", ".."}:
        raise ReleaseError("release output must name a file")
    output.parent.mkdir(parents=True, exist_ok=True)
    parent = output.parent.resolve(strict=True)
    output = parent / output.name
    parent_descriptor = _open_parent_directory(parent)
    temporary_name: str | None = None
    digest: str | None = None
    try:
        temporary_name, temporary_descriptor = _create_temporary(parent_descriptor)
        with os.fdopen(temporary_descriptor, "w+b") as temporary:
            digest = _write_archive(root, tree, temporary)

        if force:
            os.replace(
                temporary_name,
                output.name,
                src_dir_fd=parent_descriptor,
                dst_dir_fd=parent_descriptor,
            )
        else:
            try:
                os.link(
                    temporary_name,
                    output.name,
                    src_dir_fd=parent_descriptor,
                    dst_dir_fd=parent_descriptor,
                    follow_symlinks=False,
                )
            except FileExistsError as error:
                raise ReleaseError(f"output already exists: {output}") from error
            # The target link is already the publication. Cleanup cannot roll it back.
            try:
                os.unlink(temporary_name, dir_fd=parent_descriptor)
                temporary_name = None
            except OSError:
                pass

        try:
            _sync_directory_descriptor(parent_descriptor)
        except OSError as error:
            assert digest is not None
            raise ReleaseDurabilityError(output, digest, error) from error
        assert digest is not None
        return digest
    finally:
        if temporary_name is not None:
            try:
                os.unlink(temporary_name, dir_fd=parent_descriptor)
            except OSError:
                pass
        os.close(parent_descriptor)


def verify_release_archive(
    repository: Path, archive_path: Path, *, revision: str = "HEAD"
) -> str:
    """Verify exact tree payloads and every canonical ZIP metadata field."""

    root = repository_root(repository)
    tree = resolve_tree(root, revision)
    entries = _tree_entries(root, tree)
    expected_names = [_validated_member_path(path) for path, _ in entries]
    try:
        with zipfile.ZipFile(archive_path, mode="r") as archive:
            if archive.comment:
                raise ReleaseError("release archive comment must be empty")
            infos = archive.infolist()
            if [info.filename for info in infos] != expected_names:
                raise ReleaseError("release archive members do not exactly match the Git tree")
            for info, (path, object_id) in zip(infos, entries, strict=True):
                if (
                    info.compress_type != zipfile.ZIP_STORED
                    or info.date_time != ZIP_TIMESTAMP
                    or info.create_system != 3
                    or info.create_version != ZIP_CREATE_VERSION
                    or info.extract_version != ZIP_EXTRACT_VERSION
                    or info.flag_bits
                    != (0 if info.filename.isascii() else ZIP_UTF8_FLAG)
                    or info.external_attr != CANONICAL_MODE << 16
                    or info.internal_attr != 0
                    or info.extra
                    or info.comment
                    or info.is_dir()
                    or info.compress_size != info.file_size
                ):
                    raise ReleaseError(f"noncanonical ZIP metadata for {info.filename!r}")
                expected_payload = _git(root, "cat-file", "blob", object_id)
                if info.file_size != len(expected_payload):
                    raise ReleaseError(f"wrong release payload size for {path!r}")
                if archive.read(info) != expected_payload:
                    raise ReleaseError(f"wrong release payload for {path!r}")
    except (OSError, zipfile.BadZipFile, zipfile.LargeZipFile) as error:
        raise ReleaseError(f"cannot verify release archive: {error}") from error

    # ZipFile exposes central-directory metadata, while extractors also consume
    # local headers. Exact comparison with a freshly generated canonical stream
    # covers both copies of every metadata field and rejects trailing bytes.
    digest = hashlib.sha256()
    try:
        with tempfile.TemporaryFile(mode="w+b") as canonical:
            canonical_digest = _write_archive(root, tree, canonical)
            canonical.seek(0)
            with Path(archive_path).open("rb") as archive_bytes:
                while True:
                    expected_chunk = canonical.read(1024 * 1024)
                    actual_chunk = archive_bytes.read(1024 * 1024)
                    digest.update(actual_chunk)
                    if actual_chunk != expected_chunk:
                        raise ReleaseError(
                            "release archive bytes are not the canonical stored ZIP"
                        )
                    if not actual_chunk:
                        break
    except OSError as error:
        raise ReleaseError(f"cannot verify release archive bytes: {error}") from error
    actual_digest = digest.hexdigest()
    if actual_digest != canonical_digest:
        raise ReleaseError("release archive digest does not match canonical output")
    return actual_digest


def _parse_arguments(arguments: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="release ZIP destination")
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--revision", default="HEAD", help="Git tree-ish to package")
    parser.add_argument("--force", action="store_true", help="atomically replace OUTPUT")
    parser.add_argument(
        "--verify", action="store_true", help="verify the published ZIP against the Git tree"
    )
    return parser.parse_args(arguments)


def main(arguments: list[str] | None = None) -> int:
    options = _parse_arguments(sys.argv[1:] if arguments is None else arguments)
    try:
        root = repository_root(options.repository)
        # Resolve a potentially moving ref once. Both publication and optional
        # verification then consume the same immutable tree object ID.
        tree = resolve_tree(root, options.revision)
        digest = build_release(
            root,
            options.output,
            revision=tree,
            force=options.force,
        )
        if options.verify:
            verified = verify_release_archive(
                root, options.output, revision=tree
            )
            if verified != digest:
                raise ReleaseError("published archive hash changed before verification")
    except (ReleaseError, OSError, UnicodeError, zipfile.LargeZipFile) as error:
        print(f"release error: {error}", file=sys.stderr)
        return 1
    print(f"{digest}  {options.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
