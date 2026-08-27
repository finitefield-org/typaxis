# Machine package fixtures

This directory is the runnable public machine-profile fixture bundle.
`profiles/paragraph-1` contains blank 1.0/1.1/1.2 packages and the frozen
paragraph combined package. `profiles/basic-document-1/combined` is the
contract 1.2 all-advertised package for the public basic profile. `invalid/`
and `scenarios/` contain typed failure expectations. The matrices bind every
expectation exactly once to the profile closure tests; `m2-basic.json` is also
the basic-document release-verifier input.

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
cd samples/machine-package/profiles/basic-document-1/combined

"$REPOSITORY/workspace/target/debug/typaxis" check-package \
  job/document-package.json \
  --package-root job --resource-root job \
  --profile typaxis.machine-pdf/basic-document-1 \
  --emit-diagnostics "$REPOSITORY/target/machine-sample/check-diagnostics.json"

"$REPOSITORY/workspace/target/debug/typaxis" build-package \
  job/document-package.json \
  -o "$REPOSITORY/target/machine-sample/output.pdf" \
  --package-root job --resource-root job \
  --profile typaxis.machine-pdf/basic-document-1 \
  --trace "$REPOSITORY/target/machine-sample/trace.json" --trace-text \
  --emit-build-manifest "$REPOSITORY/target/machine-sample/manifest.json" \
  --emit-diagnostics "$REPOSITORY/target/machine-sample/diagnostics.json"
```

Expected artifacts are a two-page `output.pdf`, a Schema-valid trace, a built
manifest with `input_profile = typaxis.machine-pdf/basic-document-1`, matching
profile-receipt and flow-registry hashes, and empty canonical diagnostics.
Poppler-normalized text is exactly `Basic document internal external First item
Second entry PNG caption`. `check-package` emits only its empty diagnostics and
does not create PDF, trace, or manifest artifacts.

The compiled descriptor is independently available with:

```text
workspace/target/debug/typaxis capabilities --format json
```

Its bytes MUST equal `samples/machine-package/capabilities.json`, advertise
exactly `basic-document-1` and `paragraph-1` in canonical order, retain
`paragraph-1` as the default, and validate against
`schemas/machine-capabilities.schema.json`.

The focused MI2-03 slice fixture is under
`staging/basic-document-1/machine-block-styles/`. It is consumed by crate unit
tests and `schemas/validate.py`; there is no dedicated CLI runner for it. Its coverage
table binds all eight additive properties to their layout, Display, PDF, and
selected-manifest observations, including fixed-point boundaries, cascade,
page-split, and PDF paint placement cases.

The focused MI2-04 fixture is under
`staging/basic-document-1/machine-list/`. Its package combines ordered,
unordered, nested, page-split, and exact-placement cases; the selected-state
file is the slice test's canonical manifest JCS golden. The
expectation also fixes single-list, empty-painted-item, marker overflow,
exact/max+1 marker limits, deterministic double-build, and closure-tamper
coverage. Public release coverage is the complete combined package above.

The focused MI2-05 fixture is under
`staging/basic-document-1/machine-page-break/`. Its four forced boundaries
cover leading, middle, consecutive, and trailing behavior and produce five
pages with blank indexes 0, 2, and 4. The canonical trace and selected-manifest
goldens retain each before/after FlowId cursor and produced page index; focused tests
also cover exact/max+1 page limits, break-derived Display paint rejection, and
cursor tampering.

The focused MI2-06 fixture is under
`staging/basic-document-1/machine-figure/`. The resource declaration deliberately
uses `figure.data`; `figure.data.hex` is the checked-in binary-safe encoding of
the exact PNG bytes, so neither a URI suffix nor a caller media string can attest
the format. The slice test decodes those bytes through stable-read
admission, computes a 40-by-20 fixed-point placement from the 2-by-1 pixels,
splits two caption blocks onto page two, emits exactly one DrawImage, finalizes
the palette transparency as an image plus soft-mask XObject, and compares the
PDF-derived manifest with the canonical selected-state golden. Focused cases
cover caption keep, terminal oversize, hash/format/dimension/pixel-limit errors,
missing/extra/wrong IDs and XObjects, failed publication, and deterministic
double build.

The focused MI2-07 fixture is under
`staging/basic-document-1/machine-link/`. Its one selected paragraph combines a
package-anchor internal link with an uppercase-scheme external URI whose text
wraps across two selected lines. `body.ttf.hex` is the binary-safe copy of the
exact synthetic TrueType resource used by the slice test. The
canonical selected-state golden binds both logical cluster ranges, three
page-local rectangles, the normalized `https` target, one named destination,
three indirect PDF Link annotations, and the final PDF hash. Focused cases
cover empty/unpainted links, bad schemes and targets before layout, package
receipt substitution, exact rectangle/object limits, every annotation closure
tamper, and deterministic double build.

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
  --matrix samples/machine-package/matrices/m2-basic.json \
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
