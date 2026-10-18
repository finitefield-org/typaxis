#!/usr/bin/env python3
"""Run private Book 2 CLI admission controls without building its full test binary.

The three existing tests and pinned fixture are extracted from their current
sources. The real non-test CLI module tree is compiled without rewriting it.
No public profile or full-book acceptance is granted by these controls.
An audited retained compiler bundle can be used when its indexed library
sources still match. This mode does not check or build the full CLI executable.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import tarfile
import time

CASES = ("cff", "pdf", "limits", "original-cff-pdf", "japanese-cff-pdf", "jstf", "reads", "owned", "ledger", "aliases", "svg-source", "svg-clips", "paths", "catalog", "resolver", "font-media", "font-media-errors", "math-ties", "math-cff-pdf", "native-math-cff-pdf", "font-binding")
ORIGINAL_SHA = "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def row(path):
    return {"path": str(path), "byte_length": path.stat().st_size, "sha256": sha(path)}


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def source_inputs(root):
    names = {root / "workspace/Cargo.toml", root / "workspace/Cargo.lock"}
    for base in ("workspace/crates", "workspace/vendor"):
        for path in (root / base).rglob("*"):
            if not path.is_file() or any(p in ("target", ".git", "__pycache__") for p in path.relative_to(root / base).parts):
                continue
            if base.endswith("vendor") or path.suffix in (".rs", ".toml", ".lock"):
                names.add(path)
    fixture = root / "samples/machine-package/staging/production-book-1"
    names.update(fixture / "semantic-container/job" / name for name in
                 ("document-package.json", "body.bin", "collection.bin", "cover.bin"))
    names.add(fixture / "vmb-book/fixture-fonts/typaxis-cff-fixture.otf")
    names.update(root / "tools" / name for name in
                 ("verify_vmb_initial_admission.py", "vmb_book_initial_admission_probe.rs"))
    return {str(p.relative_to(root)): {"byte_length": p.stat().st_size, "sha256": sha(p)}
            for p in sorted(names)}


def run(output, role, argv, root, env, seconds):
    started = time.time()
    stdout, stderr = output / (role + ".stdout"), output / (role + ".stderr")
    record = {"argv": argv, "cwd": str(root), "timeout_seconds": seconds,
              "started_unix": started, "state": "running"}
    path = output / (role + ".command.json")
    with stdout.open("xb") as out, stderr.open("xb") as err:
        child = subprocess.Popen(argv, cwd=root, env=env, stdout=out, stderr=err,
                                 start_new_session=True)
        record["pid"] = child.pid
        write(path, record)
        timed_out = False
        def interrupted(signum, frame):
            raise InterruptedError("external runtime deadline")
        previous_handler = signal.signal(signal.SIGTERM, interrupted)
        try:
            try:
                code = child.wait(timeout=seconds)
            except (subprocess.TimeoutExpired, InterruptedError):
                # A second external deadline must not interrupt child cleanup
                # and leave a live process or a stale running record behind.
                signal.signal(signal.SIGTERM, signal.SIG_IGN)
                timed_out = True
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                child.wait()
                code = 124
        finally:
            signal.signal(signal.SIGTERM, previous_handler)
    record.update(state="complete", exit_code=code, timed_out=timed_out,
                  elapsed_seconds=time.time() - started, stdout=row(stdout), stderr=row(stderr))
    write(path, record)
    if code:
        raise RuntimeError(f"{role} exited {code}: {stderr.read_text(errors='replace')[-2000:]}")
    return record


def extract_harness(root, output):
    src = root / "workspace/crates/typaxis-cli/src"
    fixture = (src / "book_v2_column_converged_pdf_tests.rs").read_text()
    lines = fixture.splitlines(keepends=True)
    begin = [i for i, line in enumerate(lines) if line.rstrip("\n") == "// BEGIN PINNED FIXTURE-ONLY HARNESS"]
    end = [i for i, line in enumerate(lines) if line.rstrip("\n") == "// END PINNED FIXTURE-ONLY HARNESS"]
    if len(begin) != 1 or len(end) != 1 or begin[0] >= end[0]:
        raise ValueError("pinned fixture markers changed")
    fixture = "".join(lines[begin[0] + 1:end[0]])
    (output / "fixture_harness.rs").write_text(fixture)
    original = (src / "book_v2_column_initial_style.rs").read_text()
    marker = "#[cfg(test)]\nmod tests {\n"
    if original.count(marker) != 1:
        raise ValueError("initial-style test module changed")
    tests = original.split(marker, 1)[1].rsplit("\n}", 1)[0]
    if tests.count("#[test]") != 3 or tests.count("    use super::*;\n") != 1:
        raise ValueError("expected the three original initial-admission tests")
    tests = tests.replace("    use super::*;\n", "", 1)
    tests = "".join(line for line in tests.splitlines(keepends=True)
                    if not line.strip().startswith(("#[test]", "#[ignore")))
    (output / "original_initial_style_tests.rs").write_text(tests)
    template = root / "tools/vmb_book_initial_admission_probe.rs"
    source = template.read_text()
    replacements = {
        "__BOOK_MODULE__": src / "book_v2_resources.rs",
        "__TABLE_CASES__": root / "workspace/crates/typaxis-font/src/cff_font_tables_cases.rs",
        "__JSTF_CASES__": root / "workspace/crates/typaxis-font/src/cff_jstf_cases.rs",
        "__TINY_CFF__": root / "samples/machine-package/staging/production-book-1/vmb-book/fixture-fonts/typaxis-cff-fixture.otf",
    }
    for name, path in replacements.items():
        if source.count('"' + name + '"') != 1:
            raise ValueError(f"probe path marker changed: {name}")
        source = source.replace('"' + name + '"', json.dumps(str(path), ensure_ascii=False))
    (output / "probe.rs").write_text(source)
    return {"fixture_source": row(src / "book_v2_column_converged_pdf_tests.rs"),
            "tests_source": row(src / "book_v2_column_initial_style.rs"),
            "template": row(template), "fixture_extraction": row(output / "fixture_harness.rs"),
            "test_extraction": row(output / "original_initial_style_tests.rs"),
            "production_module_rewritten": False, "original_test_functions": 3}


def dependencies(path):
    result = {}
    for line in path.read_text().splitlines():
        try:
            value = json.loads(line)
        except ValueError:
            continue
        if value.get("reason") != "compiler-artifact":
            continue
        # Rust metadata can re-export procedural macros even when the harness
        # does not invoke them. Retain their dynamic libraries for restoration.
        files = [Path(p) for p in value["filenames"]
                 if p.endswith(".rlib") or ("proc-macro" in value["target"]["kind"]
                                            and p.endswith((".so", ".dylib", ".dll")))]
        if not files:
            continue
        name = value["target"]["name"]
        candidate = {**row(files[0]), "package_id": value["package_id"], "features": value["features"]}
        if name in result and result[name] != candidate:
            raise ValueError(f"ambiguous compiler dependency: {name}")
        result[name] = candidate
    if not {"typaxis_font", "typaxis_shaping", "typaxis_pdf", "serde_json"} <= result.keys():
        raise ValueError("Cargo did not provide the required staging dependencies")
    return result


def retained_dependencies(root, manifest_path, source_index_path, current):
    """Reject stale or damaged retained inputs before invoking a compiler."""
    manifest_path, source_index_path = manifest_path.resolve(), source_index_path.resolve()
    bundle_path = manifest_path.parent / "artifact-index.json"
    bundle = json.loads(bundle_path.read_text())
    if bundle.get("source_sha256") != sha(source_index_path):
        raise ValueError("retained compiler source index does not match its evidence bundle")
    recorded = [item for item in bundle.get("artifacts", {}).values()
                if Path(item["path"]).resolve() == manifest_path]
    if len(recorded) != 1 or any(recorded[0][key] != row(manifest_path)[key]
                                for key in ("byte_length", "sha256")):
        raise ValueError("retained compiler manifest does not match its evidence bundle")
    manifest = json.loads(manifest_path.read_text())
    deps = manifest.get("files", {})
    required = {"typaxis_font", "typaxis_shaping", "typaxis_pdf", "typaxis_resources",
                "typaxis_host_admission", "typaxis_resource_admission", "serde_json"}
    if manifest.get("passed") is not True or not required <= deps.keys():
        raise ValueError("retained compiler manifest lacks the required production libraries")
    for name in ("typaxis_shaping", "typaxis_pdf", "typaxis_resources"):
        if "book-v2-staging" not in deps[name].get("features", []):
            raise ValueError(f"retained compiler lacks book-v2-staging: {name}")

    def library_input(name):
        return name.startswith(("workspace/crates/", "workspace/vendor/")) or name in (
            "workspace/Cargo.toml", "workspace/Cargo.lock")

    indexed = {name: digest for name, digest in json.loads(source_index_path.read_text()).items()
               if library_input(name)}
    if not {name for name in current if library_input(name)} <= indexed.keys():
        raise ValueError("retained compiler source index omits current library inputs")
    for name, digest in indexed.items():
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError("retained compiler source index has a nonlocal path")
        path = root / relative
        if not path.is_file() or sha(path) != digest:
            raise ValueError(f"retained compiler library source changed: {name}")
    for name, item in deps.items():
        path = Path(item["path"])
        if not path.is_file() or row(path) != {key: item[key] for key in (
                "path", "byte_length", "sha256")}:
            raise ValueError(f"retained compiler library bytes changed: {name}")
    return deps, {"kind": "retained_libraries", "manifest": row(manifest_path),
                  "source_index": row(source_index_path), "evidence_index": row(bundle_path),
                  "library_source_inputs": len(indexed), "cargo_build_checked": False,
                  "full_cli_executable_built": False}


def archive(path, rows):
    expected = {}
    with path.open("xb") as output:
        with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0, compresslevel=1) as zipped:
            with tarfile.open(fileobj=zipped, mode="w|", dereference=True) as tar:
                for name, item in sorted(rows.items()):
                    source = Path(item["path"])
                    if row(source) != {k: item[k] for k in ("path", "byte_length", "sha256")}:
                        raise RuntimeError(f"archive input changed: {source}")
                    info = tar.gettarinfo(str(source), arcname=name)
                    info.uid = info.gid = info.mtime = 0
                    info.uname = info.gname = ""
                    with source.open("rb") as stream:
                        tar.addfile(info, stream)
                    expected[name] = (item["byte_length"], item["sha256"])
    with tarfile.open(path) as tar:
        actual = {m.name: (m.size, hashlib.file_digest(tar.extractfile(m), "sha256").hexdigest())
                  for m in tar if m.isfile()}
    if actual != expected:
        raise RuntimeError("compiler/source archive readback differs")
    return {**row(path), "files": len(rows), "read_back_verified": True, "gzip_compression_level": 1}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--output", type=Path, required=True, help="new evidence directory")
    parser.add_argument("--font-list", type=Path, required=True, help="original Mincho must be the first font")
    compiler = parser.add_mutually_exclusive_group(required=True)
    compiler.add_argument("--cargo-target-dir", type=Path, help="explicit managed compiler cache")
    compiler.add_argument("--compiler-manifest", type=Path, help="retained manifest beside its artifact-index.json")
    parser.add_argument("--compiler-source-index", type=Path, help="source index recorded in the retained evidence bundle")
    parser.add_argument("--case", choices=("all", *CASES), action="append", help="repeat to select several short controls; default all")
    parser.add_argument("--timeout-seconds", type=int, choices=range(1, 56), default=55)
    parser.add_argument("--compile-timeout-seconds", type=int, choices=range(1, 601),
                        help="compilation deadline only; runtime cases retain --timeout-seconds")
    args = parser.parse_args(argv)
    if bool(args.compiler_manifest) != bool(args.compiler_source_index):
        parser.error("--compiler-manifest and --compiler-source-index must be used together")
    compile_seconds = args.compile_timeout_seconds or args.timeout_seconds
    root, output = args.root.resolve(), args.output.resolve()
    if output.exists():
        parser.error("output already exists")
    if any(output.is_relative_to(root / name) for name in ("tools", "workspace/crates", "workspace/vendor")):
        parser.error("output must be outside source directories")
    paths = [Path(line.strip()) for line in args.font_list.resolve().read_text().splitlines() if line.strip()]
    paths = [(p if p.is_absolute() else args.font_list.resolve().parent / p).resolve() for p in paths]
    if not paths or sha(paths[0]) != ORIGINAL_SHA:
        parser.error("first font does not match the unchanged original Mincho hash")
    output.mkdir(parents=True)
    original = {"list": row(args.font_list.resolve()), "first_font": row(paths[0])}
    resolved_list = output / "original-fonts.txt"
    resolved_list.write_text("".join(str(p) + "\n" for p in paths))
    before = source_inputs(root)
    write(output / "source-before.json", before)
    env = {k: v for k, v in os.environ.items() if not k.startswith(("TYPAXIS_", "VMB_BOOK_"))}
    env.pop("RUSTFLAGS", None)
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    env.update(CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", CARGO_INCREMENTAL="0",
               CARGO_BUILD_JOBS="2",
               CARGO_MANIFEST_DIR=str(root / "workspace/crates/typaxis-cli"),
               PYTHONDONTWRITEBYTECODE="1", TYPAXIS_HARFRUST_CACHE_FONTS=str(resolved_list),
               TYPAXIS_HARANO_FONT=str(paths[0]))
    if args.cargo_target_dir:
        env["CARGO_TARGET_DIR"] = str(args.cargo_target_dir.resolve())
    else:
        env.pop("CARGO_TARGET_DIR", None)
    report = {"schema_version": 1, "classification": "private_cli_runtime_control",
              "passed": False, "full_book_acceptance": False, "public_profile_acceptance": False,
              "compile_timeout_seconds": compile_seconds, "runtime_timeout_seconds": args.timeout_seconds,
              "source_index_sha256": sha(output / "source-before.json"), "original": original}
    try:
        if args.compiler_manifest:
            deps, provenance = retained_dependencies(
                root, args.compiler_manifest, args.compiler_source_index, before)
            report["compiler_provenance"] = provenance
        else:
            run(output, "cargo-artifacts", ["cargo", "build", "--manifest-path", str(root / "workspace/Cargo.toml"),
                "-p", "typaxis-cli", "--features", "book-v2-staging", "--bin", "typaxis", "--offline", "--locked",
                "--message-format=json-render-diagnostics"], root, env, compile_seconds)
            deps = dependencies(output / "cargo-artifacts.stdout")
            report["compiler_provenance"] = {"kind": "cargo_build", "cargo_build_checked": True}
        report["extraction"] = extract_harness(root, output)
        write(output / "compiler-dependencies.json", deps)
        binary = output / "probe"
        command = ["rustc", "--edition=2021", "--crate-type=bin", "--cfg", 'feature="book-v2-staging"',
                   "-C", "debuginfo=0", "-C", "overflow-checks=on", "-C", "codegen-units=256"]
        for directory in sorted({str(Path(item["path"]).parent) for item in deps.values()}):
            command += ["-L", "dependency=" + directory]
        for name, item in sorted(deps.items()):
            command += ["--extern", name + "=" + item["path"]]
        command += ["--emit=link,dep-info=" + str(output / "probe.d"), str(output / "probe.rs"), "-o", str(binary)]
        report["compile"] = run(output, "probe-compile", command, root, env, compile_seconds)
        report["binary"] = row(binary)
        selected = CASES if not args.case or "all" in args.case else tuple(dict.fromkeys(args.case))
        facts = {}
        report["cases"] = facts
        for case in selected:
            command = [str(binary), case]
            if case.endswith("cff-pdf"):
                command.append(str(output / (case + ".pdf")))
            record = run(output, case, command, root, env, args.timeout_seconds)
            value = json.loads((output / (case + ".stdout")).read_text().splitlines()[-1])
            if value.get("case") != case or value.get("passed") is not True:
                raise RuntimeError(f"runtime case did not verify itself: {case}")
            facts[case] = {**value, "elapsed_seconds": record["elapsed_seconds"]}
            write(output / "report.json", report)
        report["cases"] = facts
        report["compiler_inputs"] = archive(output / "compiler-inputs.tar.gz",
            {Path(v["path"]).name: v for v in deps.values()})
        report["source_inputs"] = archive(output / "source-inputs.tar.gz",
            {n: {"path": str(root / n), **v} for n, v in before.items()})
        report["passed"] = True
    except Exception as error:
        report["error"] = str(error)
        raise
    finally:
        after = source_inputs(root)
        report["source_drift"] = sorted(n for n in before.keys() | after.keys() if before.get(n) != after.get(n))
        if report["source_drift"]:
            report["passed"] = False
        if args.compiler_manifest and "compiler_provenance" in report:
            try:
                retained_dependencies(root, args.compiler_manifest, args.compiler_source_index, after)
            except Exception as error:
                report["passed"] = False
                report["retained_compiler_drift"] = str(error)
        if sha(paths[0]) != original["first_font"]["sha256"]:
            report["passed"] = False
            report["original_font_drift"] = True
        write(output / "report.json", report)
    if not report["passed"]:
        raise SystemExit("runtime verification failed; retained report identifies the failure")
    print(json.dumps({"passed": True, "cases": len(report["cases"]), "report": str(output / "report.json")}, indent=2))


if __name__ == "__main__":
    main()
