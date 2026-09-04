# Machine package fixtures

This directory is the runnable public machine-profile fixture bundle.
`profiles/paragraph-1` contains blank 1.0/1.1/1.2 packages and the frozen
paragraph combined package. `profiles/basic-document-1/combined` is the
contract 1.2 all-advertised package for the public basic profile.
`profiles/table-1/only` and `profiles/table-1/combined` cover the table-only and
complete M2-plus-table domains. `profiles/footnote-1/zero` and
`profiles/footnote-1/combined` cover empty assignment and complete
M2-plus-footnote domains. `profiles/columns-1/combined`,
`profiles/float-1/combined`, and `profiles/header-footer-1/combined` are the
contract 1.3 all-advertised advanced packages. `invalid/` and `scenarios/` contain typed
failure expectations. The matrices bind every expectation exactly once to the
profile closure tests; `m2-basic.json`, `m3-table.json`, `m3-footnote.json`,
the three focused advanced matrices, and aggregate `m3-all.json` are the
corresponding public release-verifier inputs.

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
exactly `basic-document-1`, `columns-1`, `float-1`, `footnote-1`,
`header-footer-1`, `paragraph-1`, and `table-1` in canonical order, retain
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

The public MI3-04 fixtures are under `profiles/table-1/`. `only` fixes the
single-page table baseline. `combined` uses every M2 feature plus fixed and
fraction columns, colspan/rowspan, a split body row, and a repeated header over
three final PDF pages. Its trace and manifest contain byte-identical selected
table facts, while Display/PDF closure binds each retained cell glyph command
and rejects path decoration. Poppler-normalized combined text is exactly
`Basic document internal external First item Second entry PNG caption Header A
Header B alpha beta Header A delta Header B gamma`. Older-profile and table-style
rejections live under `invalid/`; `matrices/m3-table.json` registers the full
public table gate.

The public MI3-07 fixtures are under `profiles/footnote-1/`. `zero` proves
that a Footnote master with no references reserves and paints nothing.
`combined` uses every M2 feature plus catalog order distinct from
first-reference order, repeated references, paragraph/heading definitions,
definition anchor/page-reference/break/internal-link content, two dedicated
carry edges, and a final carry-only page. Its trace and manifest contain
byte-identical selected footnote facts, while Display/PDF closure binds body
markers, one separator per nonempty page, definition glyphs, named
destinations, and annotations. Poppler-normalized combined text is exactly
`Basic document internal external First item Second entry Z first A note A
tail PNG caption Z second Z third Z fourth Z fifth`.

The public MI3-12 advanced fixtures each inherit the complete M2 advertised
coverage on contract 1.3. `columns-1/combined` exercises two sequential
columns and selects its second exact final-page balance candidate.
`float-1/combined` exercises FIFO here/top placement, column/page carry, an
exact queue maximum of 33, and a carry maximum of 2. `header-footer-1/combined`
selects first, left, and right masters across three pages and binds each
header/body/footer repetition in paint and extraction order. Their trace and
manifest `advanced_pagination` members are byte-identical. The focused
matrices and aggregate `matrices/m3-all.json` drive the normal public CLI;
there is no private advanced runner or hidden selector.

The private MI4-02 slice is under
`staging/production-book-1/semantic-container/`. Its contract-1.4 package
contains result, proof, and exercise owners with nested page splitting, and
declares PNG, standalone TrueType sfnt, and TrueType collection bytes through
opaque `.bin` paths. `staging-semantic-container.json` is canonical JCS and
binds the selected fragments, typed style, Display paint, PDF/raster
observations, and separate declared/decoder-attested media facts. Crate tests
and `schemas/validate.py` consume this slice; no public CLI selector accepts
contract 1.4 or `production-book-1` before MI4-13.

The private MI4-V19 publication-readiness inputs are directly under
`staging/production-book-1/`. `publication-capabilities.json` freezes the exact
future eight-profile descriptor while leaving the public seven-profile bytes
unchanged; `publication-expectation.json` binds all 73 source/resource files
and every advertised production field; `external-tool-policy.json` pins
MuPDF 1.28.2, Poppler 26.08.0, and signed veraPDF 1.30.2 inputs; and
`matterhorn-assessment.json` records every Matterhorn Protocol 1.02 item with
its published detection method and an explicit passed or justified-N/A result.
The veraPDF policy pins the signed installer and the canonical installed-tree
payload, so matching version text alone cannot satisfy the host gate.
Generated component proofs, receipts, PDFs, and per-host records stay below
`target/machine-e2e/` and are not public fixtures.

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

The exact current-host footnote gate is:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m3-footnote.json \
  --runs 2 --require-external-tools
```

The table gate uses `--matrix samples/machine-package/matrices/m3-table.json`
with the same options:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m3-table.json \
  --runs 2 --require-external-tools
```

The frozen basic-document compatibility gate uses
`--matrix samples/machine-package/matrices/m2-basic.json` with the same options.

The complete M3 publication gate, including table, footnote, and all three
advanced combined fixtures, is:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m3-all.json \
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

The MI4-V19 feature-local gate is separate from the public M3 gate:

```text
TYPAXIS_VERAPDF=/path/to/verapdf-1.30.2/verapdf \
cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli \
  machine_precomposed_vector_external --locked -- --ignored
python3 tools/verify_precomposed_vector.py --repository . \
  --require-host-evidence target/machine-e2e/precomposed-vector-host-evidence \
  --required-host macos --required-host linux
```

The first command must run on independently managed macOS and Linux hosts at
the same committed source revision with the pinned tool policy. It produces
one canonical host record per target triple and fails instead of skipping when
any renderer, extractor, validator, resource proof, or manual assessment is
missing or stale.

`tools/Dockerfile.precomposed-vector-evidence` is the reproducible Linux host
definition: its Rust base image is digest-pinned and its MuPDF, Poppler, and
FreeType source archives are hash-pinned. The signed veraPDF installation is
mounted read-only and must match the policy's installed-tree payload hash;
setting only a matching version string is insufficient.
