#!/usr/bin/env python3
"""Tests for the independent MI4-V18 precomposed-vector verifier."""

from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from tools import verify_precomposed_vector as verifier


ROOT = Path(__file__).resolve().parents[1]
EXPECTATION = (
    ROOT
    / "samples/machine-package/staging/production-book-1/precomposed-vector/expected.json"
)
CORPUS = EXPECTATION.parent


def write_json(path: Path, value: object) -> None:
    path.write_bytes(verifier.canonical_json_bytes(value) + b"\n")


def reindex(directory: Path) -> None:
    records = []
    for name in sorted(verifier.EXPECTED_ARTIFACTS):
        payload = (directory / name).read_bytes()
        records.append(
            {"bytes": len(payload), "name": name, "sha256": verifier._sha256(payload)}
        )
    write_json(
        directory / "artifact-index.json",
        {"artifacts": records, "contract": verifier.ARTIFACT_CONTRACT},
    )


class PrecomposedVectorVerifierTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls._artifact_temporary = tempfile.TemporaryDirectory()
        cls.artifacts = Path(cls._artifact_temporary.name) / "precomposed-vector"
        environment = os.environ.copy()
        environment["TYPAXIS_PRECOMPOSED_VECTOR_ARTIFACT_DIR"] = str(cls.artifacts)
        completed = subprocess.run(
            [
                "cargo",
                "test",
                "--manifest-path",
                str(ROOT / "workspace/Cargo.toml"),
                "--package",
                "typaxis-cli",
                "machine_precomposed_vector_closes_private",
                "--locked",
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            check=False,
        )
        if completed.returncode != 0:
            raise AssertionError(
                "private runner failed:\n"
                + completed.stderr.decode("utf-8", "replace")
                + completed.stdout.decode("utf-8", "replace")
            )
        if not cls.artifacts.is_dir():
            raise AssertionError("private runner did not publish generated artifacts")

    @classmethod
    def tearDownClass(cls) -> None:
        cls._artifact_temporary.cleanup()

    def copied_artifacts(self, temporary: str) -> Path:
        target = Path(temporary) / "artifacts"
        shutil.copytree(self.artifacts, target)
        return target

    def test_generated_artifacts_pass_all_independent_checks(self) -> None:
        result = verifier.verify_artifacts(self.artifacts, EXPECTATION, ROOT)
        self.assertEqual(
            {check["name"] for check in result["checks"]}, verifier.REQUIRED_CHECKS
        )
        self.assertRegex(result["artifact_set_sha256"], r"^[0-9a-f]{64}$")
        self.assertEqual(len(result["artifact_records"]), 20)

    def test_unindexed_or_hash_tampered_artifact_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            with (artifacts / "output.pdf").open("ab") as stream:
                stream.write(b"tamper")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "(?:byte length|hash) differs"
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_raster_marker_is_rejected_after_consistent_reindex(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            pdf_path = artifacts / "output.pdf"
            pdf = pdf_path.read_bytes().replace(
                b"/Subtype /Form", b"/Subtype/Image", 1
            )
            pdf_path.write_bytes(pdf)
            receipt_path = artifacts / "verification.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["pdf_sha256"] = verifier._sha256(pdf)
            write_json(receipt_path, receipt)
            reindex(artifacts)
            with self.assertRaisesRegex(verifier.PrecomposedVectorError, "raster"):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_classic_xref_offset_tamper_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            pdf_path = artifacts / "output.pdf"
            pdf = pdf_path.read_bytes().replace(
                b"0000000015 00000 n \n", b"0000000016 00000 n \n", 1
            )
            self.assertNotEqual(pdf, pdf_path.read_bytes())
            pdf_path.write_bytes(pdf)
            receipt_path = artifacts / "verification.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["pdf_sha256"] = verifier._sha256(pdf)
            write_json(receipt_path, receipt)
            reindex(artifacts)
            with self.assertRaisesRegex(verifier.PrecomposedVectorError, "xref offset"):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_baseline_tamper_is_rejected_across_trace_and_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            trace_path = artifacts / "inline-layout-trace.json"
            trace = json.loads(trace_path.read_bytes())
            trace["precomposed_vector_layout"]["placements"][0]["record"]["baseline_y"] += 1
            write_json(trace_path, trace)
            reindex(artifacts)
            with self.assertRaisesRegex(verifier.PrecomposedVectorError, "baseline equation"):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_duplicate_inline_trace_node_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            trace_path = artifacts / "inline-layout-trace.json"
            trace = json.loads(trace_path.read_bytes())
            trace["precomposed_vector_layout"]["placements"][1]["record"][
                "node_id"
            ] = 3
            write_json(trace_path, trace)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "inline trace nodes"
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_manifest_dependency_tamper_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            root_path = artifacts / "build-manifest-vector.json"
            root = json.loads(root_path.read_bytes())
            root["safe_vector_manifest"]["placement_count"] += 1
            write_json(root_path, root)
            reindex(artifacts)
            with self.assertRaisesRegex(verifier.PrecomposedVectorError, "fingerprint differs"):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_legacy_figure_pdf_and_manifest_must_close_together(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            pdf_path = artifacts / "figure-output.pdf"
            pdf = pdf_path.read_bytes() + b"tamper"
            pdf_path.write_bytes(pdf)
            receipt_path = artifacts / "verification.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["figure"]["pdf_sha256"] = verifier._sha256(pdf)
            write_json(receipt_path, receipt)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError,
                "figure-output.pdf is not the deterministic PDF",
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_corpus_actual_text_tamper_is_rejected_after_consistent_reindex(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            pdf_path = artifacts / "corpus-output.pdf"
            pdf = pdf_path.read_bytes()
            match = verifier._ACTUAL_TEXT.search(pdf)
            self.assertIsNotNone(match)
            assert match is not None
            encoded = bytearray(match.group(1))
            encoded[-1] = ord("0") if encoded[-1] != ord("0") else ord("1")
            pdf = pdf[: match.start(1)] + bytes(encoded) + pdf[match.end(1) :]
            pdf_path.write_bytes(pdf)
            receipt_path = artifacts / "verification.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["corpus"]["pdf_sha256"] = verifier._sha256(pdf)
            write_json(receipt_path, receipt)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "role, ActualText, Lang, or order"
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_corpus_math_flow_template_reuse_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            display_path = artifacts / "corpus-display.json"
            display = json.loads(display_path.read_bytes())
            pages = display["precomposed_vector_display"]["pages"]
            command = pages[0]["record"]["commands"][0]
            command["record"]["math_flow"]["flow_id"] += 1
            command["fingerprint"] = verifier._sha256(
                verifier.canonical_json_bytes(command["record"])
            )
            pages[0]["fingerprint"] = verifier._sha256(
                verifier.canonical_json_bytes(pages[0]["record"])
            )
            write_json(display_path, display)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "math-flow relation differs"
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_corpus_display_content_key_must_match_admission(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            display_path = artifacts / "corpus-display.json"
            display = json.loads(display_path.read_bytes())
            pages = display["precomposed_vector_display"]["pages"]
            command = pages[0]["record"]["commands"][0]
            command["record"]["content_key"]["ir_fingerprint"] = "0" * 64
            command["record"]["ir_fingerprint"] = "0" * 64
            command["fingerprint"] = verifier._sha256(
                verifier.canonical_json_bytes(command["record"])
            )
            pages[0]["fingerprint"] = verifier._sha256(
                verifier.canonical_json_bytes(pages[0]["record"])
            )
            write_json(display_path, display)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError,
                "placement/content closure differs",
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_effective_package_tamper_cannot_be_rebound_by_the_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            package_path = artifacts / "effective-document-package.json"
            package = json.loads(package_path.read_bytes())
            package["metadata"]["title"] += " tampered"
            write_json(package_path, package)
            receipt_path = artifacts / "verification.json"
            receipt = json.loads(receipt_path.read_bytes())
            receipt["effective_package_sha256"] = verifier._sha256(
                package_path.read_bytes()[:-1]
            )
            write_json(receipt_path, receipt)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError,
                "effective/semantic package manifest closure differs",
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_corpus_metric_tamper_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            corpus = Path(temporary) / "precomposed-vector"
            shutil.copytree(CORPUS, corpus)
            cases_path = corpus / "cases.tsv"
            lines = cases_path.read_text(encoding="utf-8").splitlines()
            for index, line in enumerate(lines):
                fields = line.split("\t")
                if fields[0] == "x-plus-y":
                    fields[12] = "9999999"
                    lines[index] = "\t".join(fields)
                    break
            cases_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "metric containment"
            ):
                verifier.verify_artifacts(self.artifacts, corpus / "expected.json", ROOT)

    def test_generated_corpus_admission_tamper_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            artifacts = self.copied_artifacts(temporary)
            admission_path = artifacts / "corpus-admission.json"
            admission = json.loads(admission_path.read_bytes())
            admission["candidates"][0]["aliases"][0]["expected_sha256"] = "0" * 64
            write_json(admission_path, admission)
            reindex(artifacts)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "corpus admission alias differs"
            ):
                verifier.verify_artifacts(artifacts, EXPECTATION, ROOT)

    def test_negative_ledger_requires_an_executable_owner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            corpus = Path(temporary) / "precomposed-vector"
            shutil.copytree(CORPUS, corpus)
            ledger_path = corpus / "negative-integration.tsv"
            lines = ledger_path.read_text(encoding="utf-8").splitlines()
            fields = lines[1].split("\t")
            fields[-1] = "missing_test_owner"
            lines[1] = "\t".join(fields)
            ledger_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "missing executable Rust owners"
            ):
                verifier.verify_artifacts(self.artifacts, corpus / "expected.json", ROOT)

    def test_negative_ledger_rejects_a_non_test_function_owner(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            corpus = Path(temporary) / "precomposed-vector"
            shutil.copytree(CORPUS, corpus)
            ledger_path = corpus / "negative-integration.tsv"
            lines = ledger_path.read_text(encoding="utf-8").splitlines()
            fields = lines[1].split("\t")
            fields[-1] = "config"
            lines[1] = "\t".join(fields)
            ledger_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "missing executable Rust owners"
            ):
                verifier.verify_artifacts(self.artifacts, corpus / "expected.json", ROOT)

    def test_negative_ledger_rejects_stale_phase_or_code(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            corpus = Path(temporary) / "precomposed-vector"
            shutil.copytree(CORPUS, corpus)
            ledger_path = corpus / "negative-integration.tsv"
            lines = ledger_path.read_text(encoding="utf-8").splitlines()
            for index, line in enumerate(lines):
                fields = line.split("\t")
                if fields[0] == "pdf-object-max-plus-one":
                    fields[2] = "D8101"
                    lines[index] = "\t".join(fields)
                    break
            ledger_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "negative case outcome differs"
            ):
                verifier.verify_artifacts(self.artifacts, corpus / "expected.json", ROOT)

    def test_alternative_rejects_c1_control(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            corpus = Path(temporary) / "precomposed-vector"
            shutil.copytree(CORPUS, corpus)
            cases_path = corpus / "cases.tsv"
            lines = cases_path.read_text(encoding="utf-8").splitlines()
            fields = lines[1].split("\t")
            fields[5] = "\u0080"
            lines[1] = "\t".join(fields)
            cases_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "contains a control"
            ):
                verifier.verify_artifacts(self.artifacts, corpus / "expected.json", ROOT)

    def test_assertion_trace_rejects_a_different_valid_fixture_mapping(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            corpus = Path(temporary) / "precomposed-vector"
            shutil.copytree(CORPUS, corpus)
            trace_path = corpus / "assertion-traceability.tsv"
            lines = trace_path.read_text(encoding="utf-8").splitlines()
            fields = lines[1].split("\t")
            fields[2] = "cases.tsv#similar"
            lines[1] = "\t".join(fields)
            trace_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "assertion fixture mapping differs"
            ):
                verifier.verify_artifacts(self.artifacts, corpus / "expected.json", ROOT)

    def test_host_evidence_is_schema_valid_canonical_and_aggregatable(self) -> None:
        result = verifier.verify_artifacts(self.artifacts, EXPECTATION, ROOT)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            first_path = directory / "first.json"
            first = verifier.emit_host_evidence(
                first_path, result, ROOT, EXPECTATION
            )
            self.assertEqual(
                first_path.read_bytes(), verifier.canonical_json_bytes(first) + b"\n"
            )
            second = copy.deepcopy(first)
            if first["host"]["os"] == "macos":
                second["host"] = {
                    "arch": first["host"]["arch"],
                    "os": "linux",
                    "target_triple": f"{first['host']['arch']}-unknown-linux-gnu",
                }
                required = ["macos", "linux"]
            else:
                second["host"] = {
                    "arch": first["host"]["arch"],
                    "os": "macos",
                    "target_triple": f"{first['host']['arch']}-apple-darwin",
                }
                required = ["linux", "macos"]
            write_json(directory / "second.json", second)
            aggregate = verifier.require_host_evidence(directory, required, ROOT)
            self.assertEqual(
                aggregate["artifact_set_sha256"], result["artifact_set_sha256"]
            )
            self.assertEqual(len(aggregate["hosts"]), 2)
            first["fixture"]["package_sha256"] = "0" * 64
            second["fixture"]["package_sha256"] = "0" * 64
            write_json(first_path, first)
            write_json(directory / "second.json", second)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "stale fixture identity"
            ):
                verifier.require_host_evidence(directory, required, ROOT)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "unexpected host evidence"
            ):
                verifier.require_host_evidence(directory, [required[0]], ROOT)

    def test_aggregate_requires_every_named_host_and_byte_identity(self) -> None:
        result = verifier.verify_artifacts(self.artifacts, EXPECTATION, ROOT)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            evidence = verifier.emit_host_evidence(
                directory / "only.json", result, ROOT, EXPECTATION
            )
            missing = "linux" if evidence["host"]["os"] == "macos" else "macos"
            with self.assertRaisesRegex(verifier.PrecomposedVectorError, "missing required"):
                verifier.require_host_evidence(
                    directory, [evidence["host"]["os"], missing], ROOT
                )

    def test_aggregate_rejects_mutually_consistent_stale_source_evidence(self) -> None:
        result = verifier.verify_artifacts(self.artifacts, EXPECTATION, ROOT)
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            first = verifier.emit_host_evidence(
                directory / "first.json", result, ROOT, EXPECTATION
            )
            second = copy.deepcopy(first)
            if first["host"]["os"] == "macos":
                second["host"] = {
                    "arch": first["host"]["arch"],
                    "os": "linux",
                    "target_triple": f"{first['host']['arch']}-unknown-linux-gnu",
                }
                required = ["macos", "linux"]
            else:
                second["host"] = {
                    "arch": first["host"]["arch"],
                    "os": "macos",
                    "target_triple": f"{first['host']['arch']}-apple-darwin",
                }
                required = ["linux", "macos"]
            first["source"]["snapshot_sha256"] = "0" * 64
            second["source"]["snapshot_sha256"] = "0" * 64
            write_json(directory / "first.json", first)
            write_json(directory / "second.json", second)
            with self.assertRaisesRegex(
                verifier.PrecomposedVectorError, "stale source identity"
            ):
                verifier.require_host_evidence(directory, required, ROOT)

    def test_cli_rejects_invalid_mode_combinations(self) -> None:
        self.assertEqual(
            verifier.main(
                [
                    str(self.artifacts),
                    "--repository",
                    str(ROOT),
                    "--require-host-evidence",
                    str(self.artifacts),
                    "--required-host",
                    "macos",
                ]
            ),
            1,
        )
        self.assertEqual(
            verifier.main(
                [
                    str(self.artifacts),
                    "--repository",
                    str(ROOT),
                    "--required-host",
                    "macos",
                ]
            ),
            1,
        )


if __name__ == "__main__":
    unittest.main()
