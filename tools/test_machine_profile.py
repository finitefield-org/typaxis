#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import verify_machine_profile as machine
import verify_reproducibility as reproducibility


class MachineProfileEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.repository = Path(__file__).resolve().parent.parent
        cls.revision = subprocess.run(
            ["git", "-C", cls.repository, "rev-parse", "HEAD"],
            check=True,
            stdout=subprocess.PIPE,
            text=True,
        ).stdout.strip()

    def evidence(self, host: str) -> dict[str, object]:
        digest = "0" * 64
        triple = {
            "linux": "x86_64-unknown-linux-gnu",
            "macos": "aarch64-apple-darwin",
        }[host]
        artifact_hashes = {kind: digest for kind in machine.ARTIFACT_FILES}
        return {
            "artifacts": [
                {"bytes": 1, "kind": kind, "sha256": digest}
                for kind in sorted(machine.ARTIFACT_FILES)
            ],
            "binary": {"sha256": digest, "version": "typaxis 0.1.0"},
            "checks": [
                {"name": name, "result": "passed"}
                for name in sorted(machine.REQUIRED_CHECKS)
            ],
            "contract": machine.EVIDENCE_CONTRACT,
            "fixture": {
                "expected_sha256": digest,
                "fixture_id": "paragraph-1.combined",
                "resources": [],
            },
            "host": {"arch": triple.split("-", 1)[0], "os": host, "target_triple": triple},
            "reproducibility": {
                "artifacts": artifact_hashes,
                "binary_sha256": digest,
                "binary_version": "typaxis 0.1.0",
                "revision": self.revision,
                "source_snapshot_sha256": digest,
            },
            "result": "passed",
            "source": {
                "cargo_lock_sha256": digest,
                "revision": self.revision,
                "snapshot_sha256": digest,
            },
            "tools": [
                {"name": name, "sha256": digest, "version": "test 1"}
                for name in sorted(machine.REQUIRED_TOOLS)
            ],
        }

    def write(self, directory: Path, evidence: dict[str, object]) -> Path:
        target = directory / f"{evidence['host']['target_triple']}.json"
        machine._atomic_write(target, machine.canonical_json_bytes(evidence))
        return target

    def test_aggregation_requires_two_current_successful_canonical_hosts(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            linux = self.write(directory, self.evidence("linux"))
            macos = self.write(directory, self.evidence("macos"))
            observed = machine.require_host_evidence(
                self.repository, directory, ["macos", "linux"]
            )
            self.assertEqual(observed, {"macos": macos, "linux": linux})

    def test_aggregation_rejects_missing_failed_and_stale_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            directory = Path(raw)
            self.write(directory, self.evidence("linux"))
            with self.assertRaises(machine.MachineProfileError):
                machine.require_host_evidence(
                    self.repository, directory, ["macos", "linux"]
                )

        for mutation in ("failed", "stale"):
            with tempfile.TemporaryDirectory() as raw:
                directory = Path(raw)
                linux = self.evidence("linux")
                macos = self.evidence("macos")
                if mutation == "failed":
                    macos["result"] = "failed"
                else:
                    macos["source"]["revision"] = "f" * 40
                self.write(directory, linux)
                self.write(directory, macos)
                with self.assertRaises(machine.MachineProfileError):
                    machine.require_host_evidence(
                        self.repository, directory, ["macos", "linux"]
                    )

    def test_capability_guard_rejects_unadopted_profile(self) -> None:
        capabilities = json.loads(
            (self.repository / "samples/machine-package/capabilities.json").read_bytes()
        )
        machine._assert_profile_closure(capabilities, machine.PUBLIC_PROFILES)
        future = copy.deepcopy(capabilities)
        future["machine_input"]["profiles"].append(
            {"id": "typaxis.machine-pdf/tagged-pdf-1"}
        )
        with self.assertRaises(machine.MachineProfileError):
            machine._assert_profile_closure(future, machine.PUBLIC_PROFILES)

    def test_machine_build_flags_remove_checkout_specific_symbol_paths(self) -> None:
        flags = reproducibility._machine_build_rustflags(
            Path("/tmp/source-alpha"), Path("/tmp/target-alpha")
        )
        self.assertEqual(
            flags,
            (
                "--remap-path-prefix=/tmp/source-alpha=/typaxis-source",
                "--remap-path-prefix=/tmp/target-alpha=/typaxis-target",
                "-C",
                "debuginfo=0",
                "-C",
                "strip=symbols",
            ),
        )

    def test_reproducibility_target_names_have_equal_encoded_length(self) -> None:
        names = reproducibility.MACHINE_TARGET_DIRECTORY_NAMES
        self.assertEqual(len(set(names)), len(names))
        self.assertEqual(len({len(name.encode()) for name in names}), 1)


class ProductionCommonTraceSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        repository = Path(__file__).resolve().parent.parent
        cls.validator = machine._schema_validators(repository)["layout-trace.schema.json"]

    def trace(self) -> dict[str, object]:
        digest = "a" * 64
        return {
            "contract": "typaxis.contract/1.4",
            "coordinate_unit": "pdf_point_1_65536",
            "fragment_count": 1,
            **{key: digest for key in (
                "block_layout_sha256", "flow_registry_sha256", "inline_layout_sha256",
                "profile_receipt_sha256", "selected_layout_sha256", "vector_display_sha256",
            )},
        }

    def test_native_and_table_measurements_keep_explicit_absence_and_legacy_trace(self) -> None:
        trace = self.trace()
        self.validator.validate(trace)
        for native, tables in [(None, None), ("b" * 64, None), (None, "c" * 64), ("b" * 64, "c" * 64)]:
            with self.subTest(native=native, tables=tables):
                self.validator.validate({**trace, "native_math_layout_sha256": native,
                                         "table_measurements_sha256": tables})

    def test_native_and_table_measurements_reject_incomplete_or_invalid_identity(self) -> None:
        trace = {**self.trace(), "native_math_layout_sha256": None,
                 "table_measurements_sha256": None}
        for field in ("native_math_layout_sha256", "table_measurements_sha256"):
            missing = dict(trace)
            del missing[field]
            self.assertTrue(list(self.validator.iter_errors(missing)))
            for invalid in ("", "0" * 63, 0, {}, "F" * 64):
                with self.subTest(field=field, invalid=invalid):
                    self.assertTrue(list(self.validator.iter_errors({**trace, field: invalid})))
        self.assertTrue(list(self.validator.iter_errors({**trace, "contract": "typaxis.contract/1.3"})))

    def test_completed_pass_summary_requires_both_common_fields(self) -> None:
        trace = {**self.trace(), "native_math_layout_sha256": None,
                 "table_measurements_sha256": None, "pass_count": 4, "selected_state": 4}
        self.validator.validate(trace)
        for field in ("pass_count", "selected_state"):
            missing = dict(trace)
            del missing[field]
            self.assertTrue(list(self.validator.iter_errors(missing)))
            for value in (0, 1, 65536, True, None, "4"):
                self.assertTrue(list(self.validator.iter_errors({**trace, field: value})))

    def test_completed_pass_summary_closes_against_root_and_final_selection(self) -> None:
        layout = {"status": "converged", "pass_count": 4, "selected_state": 4}
        trace = {"pass_count": 4, "selected_state": 4}
        machine._assert_production_pass_count(layout, trace)
        machine._assert_production_pass_count(layout, {})
        for invalid in ({"pass_count": 4}, {"selected_state": 4},
                        {"pass_count": 4, "selected_state": 2},
                        {"pass_count": 2, "selected_state": 2},
                        {"pass_count": True, "selected_state": True}):
            with self.assertRaises(machine.MachineProfileError):
                machine._assert_production_pass_count(layout, invalid)
        for invalid in ({**layout, "status": "fallback"},
                        {**layout, "pass_count": 2}, {**layout, "selected_state": 2}):
            with self.assertRaises(machine.MachineProfileError):
                machine._assert_production_pass_count(invalid, trace)


if __name__ == "__main__":
    unittest.main()
