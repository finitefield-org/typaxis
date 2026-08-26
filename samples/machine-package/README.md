# Machine package fixtures

This directory is the runnable M1 fixture bundle. `profiles/paragraph-1`
contains the 1.0 blank compatibility package, the current 1.1 blank package,
and the all-advertised combined source/font package. `invalid/` and
`scenarios/` contain typed failure expectations. The two files under
`matrices/` bind every expectation exactly once to the internal closure tests.

## Run the combined fixture

From the repository root, build the current binary and create a fresh output
directory:

```text
cargo build --manifest-path workspace/Cargo.toml \
  --package typaxis-cli --bin typaxis --locked
mkdir -p target/machine-sample
```

Run check and build from the fixture directory (replace `REPOSITORY` with the
absolute repository path):

```text
REPOSITORY=/absolute/path/to/typaxis
cd samples/machine-package/profiles/paragraph-1/combined

"$REPOSITORY/workspace/target/debug/typaxis" check-package \
  job/document-package.json \
  --package-root job --resource-root job \
  --emit-diagnostics "$REPOSITORY/target/machine-sample/check-diagnostics.json"

"$REPOSITORY/workspace/target/debug/typaxis" build-package \
  job/document-package.json \
  -o "$REPOSITORY/target/machine-sample/output.pdf" \
  --package-root job --resource-root job \
  --trace "$REPOSITORY/target/machine-sample/trace.json" --trace-text \
  --emit-build-manifest "$REPOSITORY/target/machine-sample/manifest.json" \
  --emit-diagnostics "$REPOSITORY/target/machine-sample/diagnostics.json"
```

Expected artifacts are one-page `output.pdf`, a Schema-valid trace, a built
manifest with `input_profile = typaxis.machine-pdf/paragraph-1`, and empty
canonical diagnostics. Poppler-normalized text is exactly
`Typaxis machine input`. `check-package` emits only its empty diagnostics and
does not create PDF, trace, or manifest artifacts.

The compiled descriptor is independently available with:

```text
workspace/target/debug/typaxis capabilities --format json
```

Its bytes MUST equal `samples/machine-package/capabilities.json` and validate
against `schemas/machine-capabilities.schema.json`.

## Regenerate hashes and expectations

The bundle is generated, not hand-rehashed. Edit the generator inputs and run:

```text
python3 samples/machine-package/generate.py
python3 schemas/validate.py
cargo test --manifest-path workspace/Cargo.toml \
  --package typaxis-cli machine --locked
```

`generate.py` replaces the generated `capabilities.json`, `profiles/`,
`invalid/`, `scenarios/`, and `matrices/` trees. It recomputes package source
length/SHA-256 declarations, font bytes/hashes, canonical `expected.json`, and
matrix paths. Review the resulting diff; do not update an expected hash without
updating the bytes that own it.

## Release and host evidence

The exact current-host gate is:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --fixture samples/machine-package/profiles/paragraph-1/combined/expected.json \
  --runs 2 --require-external-tools
```

MuPDF `mutool` and Poppler `pdfinfo`/`pdftotext` are required. On success the
only per-host evidence writer atomically creates
`target/machine-e2e/host-evidence/{target-triple}.json`.

After independently managed Linux and macOS hosts have copied their canonical
evidence into one directory, aggregate it with:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --require-host-evidence target/machine-e2e/host-evidence \
  --required-host macos --required-host linux
```

The aggregate rejects missing, failed, noncanonical, stale-revision, mismatched
source/fixture, and cross-host artifact evidence. Run the current-host command
explicitly on each managed host. This repository does not use GitHub Actions or
GitHub workflow files.
