#!/usr/bin/env python3

from __future__ import annotations

import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import verify_machine_profile as machine


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

    def test_capability_guard_rejects_post_m1_advertisement(self) -> None:
        capabilities = json.loads(
            (self.repository / "samples/machine-package/capabilities.json").read_bytes()
        )
        machine._assert_m1_only(capabilities)
        future = copy.deepcopy(capabilities)
        future["machine_input"]["profiles"][0]["blocks"].append("table")
        with self.assertRaises(machine.MachineProfileError):
            machine._assert_m1_only(future)


if __name__ == "__main__":
    unittest.main()
