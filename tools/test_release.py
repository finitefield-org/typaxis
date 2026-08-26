#!/usr/bin/env python3

from __future__ import annotations

import errno
import hashlib
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

import release
import verify_reproducibility as reproducibility


def git(repository: Path, *arguments: str) -> str:
    completed = subprocess.run(
        ["git", "-C", os.fspath(repository), *arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return completed.stdout.strip()


def commit(repository: Path, message: str = "fixture") -> str:
    git(
        repository,
        "-c",
        "user.name=Typaxis test",
        "-c",
        "user.email=typaxis-test@example.invalid",
        "commit",
        "--quiet",
        "-m",
        message,
    )
    return git(repository, "rev-parse", "HEAD^{tree}")


def git_blob(repository: Path, payload: bytes) -> str:
    completed = subprocess.run(
        ["/usr/bin/git", "-C", os.fspath(repository), "hash-object", "-w", "--stdin"],
        input=payload,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout.decode("ascii", "strict").strip()


def make_tree(
    parent: Path,
    name: str,
    *,
    symlink: bool = False,
    collision: bool = False,
    file_directory_collision: bool = False,
    unicode_collision: bool = False,
    committed: bool = False,
) -> tuple[Path, str]:
    repository = parent / name
    repository.mkdir()
    git(repository, "init", "--quiet")
    git(repository, "config", "core.ignorecase", "false")
    git(repository, "config", "core.precomposeunicode", "false")
    (repository / "README.md").write_bytes(b"typaxis\n")
    (repository / "nested").mkdir()
    (repository / "nested/data.bin").write_bytes(bytes(range(32)))
    if symlink:
        (repository / "link").symlink_to("README.md")
    git(repository, "add", "-A")
    if collision:
        # Populate the index directly so the fixture is valid on both
        # case-sensitive and case-insensitive host filesystems.
        upper = git_blob(repository, b"upper")
        lower = git_blob(repository, b"lower")
        git(repository, "update-index", "--add", "--cacheinfo", f"100644,{upper},Case")
        git(repository, "update-index", "--add", "--cacheinfo", f"100644,{lower},case")
    if file_directory_collision:
        file_blob = git_blob(repository, b"file")
        nested_blob = git_blob(repository, b"nested")
        git(repository, "update-index", "--add", "--cacheinfo", f"100644,{file_blob},A")
        git(
            repository,
            "update-index",
            "--add",
            "--cacheinfo",
            f"100644,{nested_blob},a/data",
        )
    if unicode_collision:
        nfc_blob = git_blob(repository, b"NFC")
        nfd_blob = git_blob(repository, b"NFD")
        git(
            repository,
            "update-index",
            "--add",
            "--cacheinfo",
            f"100644,{nfc_blob},\N{LATIN SMALL LETTER E WITH ACUTE}",
        )
        git(
            repository,
            "update-index",
            "--add",
            "--cacheinfo",
            f"100644,{nfd_blob},e\N{COMBINING ACUTE ACCENT}",
        )
    if committed:
        return repository, commit(repository)
    return repository, git(repository, "write-tree")


def make_fake_cargo(path: Path, pdf_payload: bytes) -> None:
    typaxis_source = "\n".join(
        [
            f"#!{sys.executable}",
            "import os",
            "from pathlib import Path",
            "import sys",
            "if any(key.upper().startswith('TYPAXIS_') for key in os.environ):",
            "    raise SystemExit(91)",
            "arguments = sys.argv[1:]",
            "if not arguments or arguments[0] != 'build' or '-o' not in arguments:",
            "    raise SystemExit(92)",
            "output = Path(arguments[arguments.index('-o') + 1])",
            "output.write_bytes(" + repr(pdf_payload) + ")",
            "",
        ]
    )
    cargo_source = "\n".join(
        [
            f"#!{sys.executable}",
            "import os",
            "from pathlib import Path",
            "if any(key.upper().startswith('TYPAXIS_') for key in os.environ):",
            "    raise SystemExit(90)",
            "target = Path(os.environ['CARGO_TARGET_DIR']) / 'debug'",
            "target.mkdir(parents=True, exist_ok=True)",
            "binary = target / 'typaxis'",
            "binary.write_text(" + repr(typaxis_source) + ", encoding='utf-8')",
            "binary.chmod(0o755)",
            "",
        ]
    )
    path.write_text(cargo_source, encoding="utf-8")
    path.chmod(0o755)


class ReleaseArchiveTests(unittest.TestCase):
    def test_different_checkout_names_produce_identical_stored_zip(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            first, first_tree = make_tree(temporary, "first-checkout")
            second, second_tree = make_tree(temporary, "unrelated-name")
            self.assertEqual(first_tree, second_tree)
            first_zip = temporary / "first.zip"
            second_zip = temporary / "second.zip"
            first_hash = release.build_release(first, first_zip, revision=first_tree)
            second_hash = release.build_release(second, second_zip, revision=second_tree)
            self.assertEqual(first_hash, second_hash)
            self.assertEqual(first_zip.read_bytes(), second_zip.read_bytes())
            self.assertEqual(first_hash, hashlib.sha256(first_zip.read_bytes()).hexdigest())
            self.assertEqual(
                release.verify_release_archive(first, first_zip, revision=first_tree),
                first_hash,
            )
            self.assertEqual(
                release.verify_release_archive(second, second_zip, revision=second_tree),
                second_hash,
            )
            self.assertEqual(
                stat.S_IMODE(first_zip.stat().st_mode), release.PUBLISHED_FILE_MODE
            )

            with zipfile.ZipFile(first_zip) as archive:
                self.assertFalse(archive.comment)
                infos = archive.infolist()
                names = [info.filename for info in infos]
                self.assertEqual(names, sorted(names, key=lambda name: name.encode("utf-8")))
                self.assertEqual(
                    names,
                    [
                        f"{release.ARCHIVE_ROOT}/README.md",
                        f"{release.ARCHIVE_ROOT}/nested/data.bin",
                    ],
                )
                for info in infos:
                    self.assertEqual(info.compress_type, zipfile.ZIP_STORED)
                    self.assertEqual(info.date_time, release.ZIP_TIMESTAMP)
                    self.assertEqual(info.create_system, 3)
                    self.assertEqual(info.create_version, release.ZIP_CREATE_VERSION)
                    self.assertEqual(info.extract_version, release.ZIP_EXTRACT_VERSION)
                    self.assertEqual(info.external_attr, release.CANONICAL_MODE << 16)
                    self.assertEqual(stat.S_IMODE(info.external_attr >> 16), 0o644)
                    self.assertEqual(info.internal_attr, 0)
                    self.assertFalse(info.extra)
                    self.assertFalse(info.comment)

    def test_release_uses_only_frozen_git_tree_and_excludes_build_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, tree = make_tree(temporary, "checkout")
            (repository / "README.md").write_bytes(b"dirty worktree\n")
            (repository / "untracked.txt").write_bytes(b"untracked\n")
            target = repository / "workspace/target/debug"
            target.mkdir(parents=True)
            (target / "typaxis").write_bytes(b"build output")
            output = repository / "dist/release.zip"

            digest = release.build_release(repository, output, revision=tree)
            self.assertEqual(
                release.verify_release_archive(repository, output, revision=tree), digest
            )
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(
                    archive.read(f"{release.ARCHIVE_ROOT}/README.md"), b"typaxis\n"
                )
                names = set(archive.namelist())
            self.assertFalse(any("target" in name.split("/") for name in names))
            self.assertFalse(any(name.endswith("untracked.txt") for name in names))
            self.assertFalse(any(name.endswith("release.zip") for name in names))

    def test_existing_output_is_preserved_without_force(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, tree = make_tree(temporary, "checkout")
            output = temporary / "release.zip"
            output.write_bytes(b"keep")
            with self.assertRaises(release.ReleaseError):
                release.build_release(repository, output, revision=tree)
            self.assertEqual(output.read_bytes(), b"keep")
            self.assertFalse(list(temporary.glob(".typaxis-release-*.tmp")))
            release.build_release(repository, output, revision=tree, force=True)
            self.assertTrue(output.read_bytes().startswith(b"PK"))

    def test_force_preserves_existing_output_on_prepublication_failure(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, tree = make_tree(temporary, "checkout")
            output = temporary / "release.zip"
            output.write_bytes(b"keep")
            with mock.patch.object(
                release,
                "_write_archive",
                side_effect=release.ReleaseError("injected write failure"),
            ):
                with self.assertRaises(release.ReleaseError):
                    release.build_release(
                        repository, output, revision=tree, force=True
                    )
            self.assertEqual(output.read_bytes(), b"keep")
            self.assertFalse(list(temporary.glob(".typaxis-release-*.tmp")))

    def test_post_publication_sync_error_reports_visible_archive(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, tree = make_tree(temporary, "checkout")
            output = temporary / "release.zip"
            error = OSError(errno.EIO, "injected directory sync failure")
            with mock.patch.object(
                release, "_sync_directory_descriptor", side_effect=error
            ):
                with self.assertRaises(release.ReleaseDurabilityError) as raised:
                    release.build_release(repository, output, revision=tree)

            self.assertEqual(raised.exception.output, output.resolve())
            self.assertEqual(raised.exception.source, error)
            self.assertTrue(output.is_file())
            self.assertEqual(
                raised.exception.digest,
                hashlib.sha256(output.read_bytes()).hexdigest(),
            )
            self.assertIn("already visible", str(raised.exception))
            self.assertFalse(list(temporary.glob(".typaxis-release-*.tmp")))

    def test_archive_verifier_rejects_noncanonical_archive(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, tree = make_tree(temporary, "checkout")
            output = temporary / "release.zip"
            release.build_release(repository, output, revision=tree)
            with zipfile.ZipFile(output, mode="a") as archive:
                archive.comment = b"ambient comment"
            with self.assertRaises(release.ReleaseError):
                release.verify_release_archive(repository, output, revision=tree)

    def test_unicode_member_uses_only_the_canonical_utf8_flag(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, _ = make_tree(temporary, "checkout")
            filename = "\N{HIRAGANA LETTER A}.txt"
            (repository / filename).write_bytes(b"unicode path\n")
            git(repository, "add", "-A")
            tree = git(repository, "write-tree")
            output = temporary / "release.zip"

            digest = release.build_release(repository, output, revision=tree)
            self.assertEqual(
                release.verify_release_archive(repository, output, revision=tree), digest
            )
            with zipfile.ZipFile(output) as archive:
                info = archive.getinfo(f"{release.ARCHIVE_ROOT}/{filename}")
            self.assertEqual(info.flag_bits, release.ZIP_UTF8_FLAG)

    def test_ambient_git_redirection_and_revision_options_are_rejected(self) -> None:
        environment = release.git_environment(
            {
                "PATH": "/bin",
                "GIT_DIR": "/attacker/git-dir",
                "git_work_tree": "/attacker/worktree",
                "GiT_CONFIG_COUNT": "1",
            }
        )
        self.assertEqual(environment["PATH"], "/bin")
        self.assertNotIn("GIT_DIR", environment)
        self.assertNotIn("git_work_tree", environment)
        self.assertNotIn("GiT_CONFIG_COUNT", environment)
        self.assertEqual(environment["GIT_CONFIG_NOSYSTEM"], "1")
        self.assertEqual(environment["GIT_CONFIG_GLOBAL"], os.devnull)
        self.assertEqual(environment["GIT_NO_REPLACE_OBJECTS"], "1")

        with tempfile.TemporaryDirectory() as raw:
            repository, _ = make_tree(Path(raw), "checkout")
            with self.assertRaises(release.ReleaseError):
                release.resolve_tree(repository, "--help")

    def test_cli_freezes_moving_revision_before_build_and_verification(self) -> None:
        root = Path("/canonical/repository")
        output = Path("release.zip")
        tree = "a" * 40
        digest = "b" * 64
        with (
            mock.patch.object(release, "repository_root", return_value=root),
            mock.patch.object(release, "resolve_tree", return_value=tree) as resolve,
            mock.patch.object(release, "build_release", return_value=digest) as build,
            mock.patch.object(
                release, "verify_release_archive", return_value=digest
            ) as verify,
            mock.patch("builtins.print") as output_line,
        ):
            result = release.main(
                [
                    os.fspath(output),
                    "--repository",
                    "source",
                    "--revision",
                    "moving-ref",
                    "--verify",
                ]
            )

        self.assertEqual(result, 0)
        resolve.assert_called_once_with(root, "moving-ref")
        build.assert_called_once_with(root, output, revision=tree, force=False)
        verify.assert_called_once_with(root, output, revision=tree)
        output_line.assert_called_once_with(f"{digest}  {output}")

    def test_unsafe_tree_entries_are_rejected(self) -> None:
        for path in [
            "",
            "/absolute",
            "../escape",
            "a/../escape",
            "a//b",
            "a/",
            "a\\b",
            "a\0b",
            "a\x1fb",
            "a\x7fb",
            ".git/config",
            "workspace/target/debug/typaxis",
            "__pycache__/release.pyc",
            "AUX.txt",
            "CON .txt",
            "trailing.",
            "trailing ",
            "alternate:data",
        ]:
            with self.subTest(path=path):
                with self.assertRaises(release.ReleaseError):
                    release._validated_member_path(path)

        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            linked, linked_tree = make_tree(temporary, "linked", symlink=True)
            with self.assertRaises(release.ReleaseError):
                release.build_release(linked, temporary / "linked.zip", revision=linked_tree)

            colliding, colliding_tree = make_tree(
                temporary, "colliding", collision=True
            )
            with self.assertRaises(release.ReleaseError):
                release.build_release(
                    colliding, temporary / "colliding.zip", revision=colliding_tree
                )

            file_directory, file_directory_tree = make_tree(
                temporary, "file-directory", file_directory_collision=True
            )
            with self.assertRaises(release.ReleaseError):
                release.build_release(
                    file_directory,
                    temporary / "file-directory.zip",
                    revision=file_directory_tree,
                )

            unicode_repository, unicode_tree = make_tree(
                temporary, "unicode", unicode_collision=True
            )
            with self.assertRaises(release.ReleaseError):
                release.build_release(
                    unicode_repository,
                    temporary / "unicode.zip",
                    revision=unicode_tree,
                )


class ReproducibilityTests(unittest.TestCase):
    def test_environment_filter_removes_all_typaxis_configuration(self) -> None:
        environment = reproducibility.filtered_environment(
            {
                "PATH": "/bin",
                "TYPAXIS_STRICT": "true",
                "typaxis_limits__max_input_bytes": "1",
                "CARGO_TARGET_DIR": "/ambient/target",
                "CARGO_ENCODED_RUSTFLAGS": "ambient",
            }
        )
        self.assertEqual(environment["PATH"], "/bin")
        self.assertFalse(
            any(key.upper().startswith("TYPAXIS_") for key in environment)
        )
        self.assertNotIn("CARGO_TARGET_DIR", environment)
        self.assertNotIn("CARGO_ENCODED_RUSTFLAGS", environment)
        self.assertEqual(environment["CARGO_INCREMENTAL"], "0")

    def test_materialized_checkout_contains_only_selected_git_tree(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository, tree = make_tree(temporary, "source", committed=True)
            build_output = repository / "workspace/target/debug/typaxis"
            build_output.parent.mkdir(parents=True)
            build_output.write_bytes(b"not source")
            (repository / "untracked.txt").write_bytes(b"not source")
            destination = temporary / "different-checkout-name"

            reproducibility._materialize_checkout(
                repository,
                destination,
                tree,
                reproducibility.filtered_environment(),
            )

            self.assertEqual((destination / "README.md").read_bytes(), b"typaxis\n")
            self.assertTrue((destination / ".git").is_dir())
            self.assertFalse((destination / "workspace/target").exists())
            self.assertFalse((destination / "untracked.txt").exists())

    def test_two_checkout_verifier_exact_compares_pdf_and_zip(self) -> None:
        pdf_payload = b"%PDF-1.7\n% deterministic blank fixture\n%%EOF\n"
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            repository = temporary / "source"
            repository.mkdir()
            git(repository, "init", "--quiet")
            (repository / "workspace").mkdir()
            (repository / "workspace/Cargo.toml").write_text(
                "[workspace]\nresolver = \"2\"\n", encoding="utf-8"
            )
            (repository / "workspace/Cargo.lock").write_text(
                "version = 3\n", encoding="utf-8"
            )
            blank = repository / "samples/minimal/empty.tsf"
            blank.parent.mkdir(parents=True)
            blank.write_bytes(b"\n")
            (repository / "README.md").write_bytes(b"fixture\n")
            git(repository, "add", "-A")
            tree = commit(repository)
            fake_cargo = temporary / "fake-cargo"
            make_fake_cargo(fake_cargo, pdf_payload)

            with mock.patch.dict(
                os.environ,
                {
                    "TYPAXIS_STRICT": "true",
                    "typaxis_limits__max_input_bytes": "1",
                    "CARGO_TARGET_DIR": os.fspath(temporary / "ambient-target"),
                },
                clear=False,
            ):
                result = reproducibility.verify_reproducibility(
                    repository, revision="HEAD", cargo=os.fspath(fake_cargo)
                )

            self.assertEqual(result.tree, tree)
            self.assertEqual(result.pdf_sha256, hashlib.sha256(pdf_payload).hexdigest())
            self.assertRegex(result.release_sha256, r"\A[0-9a-f]{64}\Z")


if __name__ == "__main__":
    unittest.main()
