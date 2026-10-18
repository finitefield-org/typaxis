# Real VMB book fixtures

`svg/chapter-fraction-equivalence.svg` is an unchanged extraction from the
user-provided VMB chapter package, resource 0. Its supplied bytes include
whitespace before `/>`; do not normalize it. `fixture-index.json` records its
hash and the producer declaration, without inventing an exporter revision.
The TeX comes from text buffer 3 (`source_tex` on node 7).

Source: `vmb-book-fractions-equivalence/v1` in the user-provided VMB repository.
The supplied-book fixtures include this small expression and its SVG output,
plus the one raster diagram described below. No complete chapter/book or font
binary is copied into this fixture. This regression
input is not a grant to redistribute the complete book or its fonts.

This is currently parser evidence. The supplied generic alt and original producer
metrics are intentionally not adopted as accessibility/placement goldens. The
full design also requires the remaining real formula cases, validated exporter
outputs, chapter/5,000-resource CLI tests, and independent full-book PDF checks.

`engine-v2/` adds 20 conversions of five test expressions (fraction,
parentheses, equivalence, negation and a long sum), rendered by the actual VMB
2.0.0 engine in inline/block mode at two physical font sizes. These are authored
test inputs, not 20 occurrences extracted from the supplied book. The index
retains each original engine SVG, derived Safe-SVG, source/derived hash,
engine/bundle identity, TeX, speech and physical metrics. Font source records and
the bundled license/notice files are retained; no font binary is copied.

Regenerate from the VMB repository's `vmb-core` directory:

```sh
go run ./tools/typaxis-math-fixtures -output NEW_DIRECTORY
```

Two complete generations were byte-identical. The public CLI admission test
checks all 20 conversions and their source/derived hashes. The mixed-text PDF
probe produced 20 corresponding math facts and 20 extracted alternatives, but
its reading order failed the source-order comparison. These files are geometry
and admission fixtures, not approved PDF visual/accessibility goldens. The
formula-only line-start case also remains a required production-layout fix.


`raster/book-venn.png` is an unchanged copy of image 39 (first source placement
node 170) in the supplied whole-book package. The raster index binds the original
package hash, image hash and 1200 × 720 dimensions. It is a regression fixture
under the user's requested VMB integration scope, not a grant to redistribute the
complete book. The test caption and two Alt strings are synthetic fixture labels;
they are not adopted as the book's authored accessibility text.

`raster/orientation-alpha.png` is a synthetic 2 × 2, 8-bit RGBA test image with
red/green/blue/yellow pixels and alpha 255/128/64/0 in row-major order. The exact
pixels and byte hash are in the index. Its asymmetric rows and alpha make an
image Y flip or missing soft mask independently detectable. Tests combine these
rasters with the actual engine's inline/block fraction SVGs, a real caption line,
following text, and an alias resource carrying a distinct Figure/Alt. The existing
`color-2x1.jpg` fixture supplies the baseline JPEG case.

`tools/verify_production_raster_probe.py` checks the diagnostic PDF's decoded
pixels against the source, source-ordered raster matrices, shared Image objects,
per-occurrence Figure/Alt/MCR/ParentTree, and rendered alpha/orientation. These
small probes do not establish public check/build, complete-book or PDF/UA success.


The authored engine input ledger added in checkpoint 304 consists of `cases.json`,
`occurrences.json`, and the exact request bytes under `tex/`. Its 20 cases retain
the five engine expressions, inline/block modes, and both original physical
sizes. Original and admitted SVG bytes are unchanged. Each occurrence points to
its original engine request record; these are authored fixture occurrences,
not extracted occurrences from the original full book.

`tools/vmb_book_inputs.py` checks every fixture file hash, complete case and
occurrence sets, exact TeX/speech, source geometry and physical metrics, and the
outer clip's five stored segments. Primary ledgers use JCS plus one LF. The
file inventory covers every regular file below this directory except its own
`fixture-index.json`; earlier diagnostic outputs remain historical evidence,
not automatically approved visual expectations.

`package-template.json` is an authored composition recipe with empty content.
The three fonts under `fixture-fonts/` are byte copies of the repository's
synthetic combined-profile test fonts. They contain empty outlines and are used
only for formula-only check/build and structural tests. They do not prove body
text visibility, Japanese typography, or body/formula spacing. Their hashes are
in the template and complete fixture inventory.

Compose an exclusive generated job outside this directory with:

```sh
python3 tools/prepare_vmb_book_fixtures.py --fixture-root \
  samples/machine-package/staging/production-book-1/vmb-book \
  --output-root workspace/target/vmb-authored-job
```

The unchanged chapter whitespace SVG remains an admission regression. The
original book's maximum-path and maximum-segment SVGs, independent layout
goldens, and the complete formal book runner remain separate required work.

## Additional thin-stroke corpus

`thin-strokes/` contains eight separately registered actual-engine fraction-rule and diagonal-negation controls. The primary twenty-case ledger and three CLI tests remain the small-engine regression corpus. The dedicated thin-stroke verifier owns the additional eight cases and their damage checks. See `thin-strokes/provenance.json` for the VMB base revision plus the retained Go overlay and source/license scope.

The `chapter-layout/` support data records an authored 400-formula chapter plan
using the unchanged 20 real-engine cases, with 200 visible ASCII equation numbers.
It preserves original DejaVu Sans bytes and their license for the number glyphs.
The plan is not an original-book chapter or an approved layout golden; the
chapter gate requires independent source, glyph, paint, baseline, and page checks.

The authored `mixed-body-spacing/` plan pairs the unchanged real-engine cases with original IPAex Mincho 004.01 body glyphs. It retains the unmodified font, its full license and readme, download provenance, and six explicit body/spacing plans. This is a separate fixture font; original Harano full-book acceptance remains open.
