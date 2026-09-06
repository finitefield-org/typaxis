#!/usr/bin/env python3
"""Verify public check/build diagnostics for the unchanged VMB Harano font.

The font is supplied locally and is never downloaded or added to the repository.
This is a negative admission gate, not a full-book or Harano support gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess


HARANO_SHA256 = "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--typaxis", type=Path, required=True)
    parser.add_argument("--font", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary, font, output = (p.resolve() for p in (args.typaxis, args.font, args.output))
    with font.open("rb") as source:
        font_bytes = source.read(128 * 1024 * 1024 + 1)
    if digest(font_bytes) != HARANO_SHA256:
        parser.error("this gate requires the recorded, unchanged VMB Harano font hash")
    # A fresh destination preserves earlier observations, including failed runs.
    output.mkdir(parents=True, exist_ok=False)
    fixture = Path(__file__).resolve().parents[1] / "samples/machine-package/profiles/production-book-1/combined/job"
    job = output / "job"
    shutil.copytree(fixture, job)
    name = "HaranoAjiMincho-Regular.otf"
    (job / name).write_bytes(font_bytes)
    package_path = job / "document-package.json"
    package = json.loads(package_path.read_text())
    package["resources"]["font_faces"][2].update(uri=name, expected_sha256=HARANO_SHA256)
    package_path.write_text(json.dumps(package, ensure_ascii=False, separators=(",", ":")))
    config = output / "verification.toml"
    config.write_text('contract = "typaxis.contract/1.4"\npdf_stream_compression = "none"\n')
    env = {key: value for key, value in os.environ.items() if not key.startswith("TYPAXIS_")}
    report = {
        "algorithm": "typaxis.vmb-harano-diagnostics/1",
        "font_sha256": HARANO_SHA256,
        "binary_sha256": digest(binary.read_bytes()),
        "package_sha256": digest(package_path.read_bytes()),
        "config_sha256": digest(config.read_bytes()),
        "harano_supported": False,
        "full_book": False,
        "runs": [],
        "passed": False,
    }
    try:
        for command in ("check-package", "build-package"):
            diagnostics = output / f"{command}-diagnostics.json"
            argv = [str(binary), command, str(package_path), "--profile",
                    "typaxis.machine-pdf/production-book-1", "--config", str(config),
                    "--package-root", str(job), "--resource-root", str(job),
                    "--emit-diagnostics", str(diagnostics)]
            if command == "build-package":
                argv += ["--output", str(output / "output.pdf"),
                         "--emit-build-manifest", str(output / "manifest.json")]
            completed = subprocess.run(argv, cwd=output, env=env, capture_output=True,
                                       text=True, timeout=120, check=False)
            observed = {"argv": argv, "exit_code": completed.returncode,
                        "stderr": completed.stderr, "stdout": completed.stdout}
            report["runs"].append(observed)
            observed["diagnostics"] = json.loads(diagnostics.read_text())
            assert completed.returncode == 1, observed
            assert completed.stderr.count("R7100") == 1, observed
            assert not (output / "output.pdf").exists()
            items = observed["diagnostics"]["diagnostics"]
            assert len(items) == 1, items
            item = items[0]
            assert item["code"] == "R7100", item
            assert item["message"] == "cff1 unsupported_table", item
            assert item["location"]["json_pointer"] == "/resources/font_faces/2", item
            assert item["location"]["byte_offset"] is None, item
            notes = item["notes"]
            assert len(notes) == 3, notes
            assert notes[0]["message"] == f"resource={name}", notes
            for marker in ("phase=sfnt-directory", "table=VORG", "font_byte=108",
                           "embedding=allowed", "fs_type=0x0000"):
                assert marker in notes[1]["message"], notes
            assert "inspect-font FONT" in notes[2]["message"], notes
        assert report["runs"][0]["diagnostics"] == report["runs"][1]["diagnostics"]
        manifest = json.loads((output / "manifest.json").read_text())
        assert manifest["status"] == "failed", manifest
        assert manifest["output"] is None, manifest
        assert digest(package_path.read_bytes()) == report["package_sha256"]
        assert digest((job / name).read_bytes()) == HARANO_SHA256
        report["manifest_sha256"] = digest((output / "manifest.json").read_bytes())
        report["passed"] = True
    finally:
        (output / "observed.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(output / "observed.json")


if __name__ == "__main__":
    main()
