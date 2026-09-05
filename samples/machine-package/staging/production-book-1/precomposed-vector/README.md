# VMB precomposed-vector corpus

This directory is the checked-in producer-interface corpus for `MI4-V01`, the
canonical contract-1.4 component Wire fixture added by `MI4-V03`, and the
generated/external evidence expectations closed by `MI4-V18` and `MI4-V19`.
The public 1.4 decoder and Schema accept `document-package.json`; the complete
public CLI release fixture remains `profiles/production-book-1/combined`.

The corpus has four canonical ledgers, two focused Wire fixtures, two canonical
selected-layout traces, and one canonical Display trace. `MI4-V13` also uses
these admitted resources through a test-only PDF contribution fixture; it does
not add a standalone production PDF receipt:

- `resources.tsv` records dense logical image IDs, contained SVG paths,
  hash-derived production URIs, exact SHA-256 values, and producer provenance.
- `cases.tsv` binds semantic cases to resources, exact TeX files, alternative
  text, nullable/forbidden actual text, language intent, fixed-point metrics,
  spacing, optional equation numbers, and coverage categories.
- `fragments.tsv` binds corpus-only document fragments to ordered case
  occurrences and optional line/block fit context. A fragment placement is
  written as `{case-id}`; this notation and its context fields are test
  evidence and are not a Typaxis source syntax.
- `negative.tsv` binds rejected SVG fixtures under `negative-svg/` to the
  closed Safe-SVG 2 `R7100` reason vocabulary. These files are parser test
  inputs and are never referenced by the document package.
- `negative-integration.tsv` is the fail-closed integration matrix. Each row
  names the owning executable Rust test together with the terminal phase,
  diagnostic code/location, phase-receipt side effects, and the only visible
  failed-build artifacts. It includes wire/profile, admission/binding,
  layout, Form/content, structure, final-PDF, manifest, and exact max+1 limit
  failures; it is evidence metadata rather than an instruction to synthesize
  a failure at verification time.
- `assertion-traceability.tsv` maps every assertion group in design §15.1–15.5
  to its checked-in fixture or generated artifact and its executable Rust or
  independent-verifier owner.
- `expected.json` is the strict production evidence-runner expectation. It
  fixes the evidence command/profile identity, successful phase side effects, page and
  normalized-extraction expectations, advertised four-kind coverage, and the
  byte length and SHA-256 of every distinct corpus SVG. It is canonical JCS
  plus one LF and conforms to the current 1.4 fixture-expectation Schema.
- `document-package.json` and `input.tsf` form the canonical strict-Wire/JCS
  fixture covering all four vector kinds, nullable actual text, source TeX,
  spacing, equation-number child shape, and one `svg-safe-2` provenance record.
  They do not authorize resource admission, layout, or PDF generation.
- `document-package-language-overrides.json` is the `MI4-V14` canonical
  positive fixture. It gives each of the four vector owners the raw BCP 47
  spelling `EN-us`; computed-language `/2` canonicalizes every value to
  `en-US`, preserves the already validated raw/canonical text charge, and
  marks all four selected placements as paint-level language candidates.
- `inline-layout-trace.json` records the selected atomic inline layout from
  `MI4-V09`. `block-layout-trace.json` records the selected atomic block layout
  from `MI4-V11`, including page/block/paint ordinals, effective spacing,
  viewport matrices, formula baselines, a separately placed equation number,
  structure-child source order, and the one-time fragment charge.
- `display-v2.json` records the `MI4-V12` DrawVector `/2` closure. Its four
  commands cover all vector kinds, are sorted by `(page_index, paint_ordinal)`,
  use dense usage IDs, and reuse one component-wise `VectorContentKey` across
  inline/block and Figure/Formula placements. The Rust fixture also admits an
  unused, distinct vector resource and proves that it emits no command and
  cannot be substituted for the selected content key.
- The `MI4-V13` resource/PDF fixture derives `typaxis.safe-vector-form-plans/2`
  and `typaxis.safe-vector-pdf-contribution/2` from the sealed Display and
  admitted candidate registry. A test-only ten-use projection proves one
  content key produces one Form and ten page `Do` operations; the distinct
  unused candidate remains in the audit fingerprint and produces no Form,
  name, object role, or `Do`. The isolated writer is assertion-only. A
  production `typaxis.safe-vector-pdf-closure/2` still requires the complete
  final writer's bytes, hash, and object/use table.
- The `MI4-V16` test-only tagged-PDF fixture starts from the same canonical
  package bytes, adds the required nonempty document title and one outline
  destination, applies the
  canonical `en-US` override to all four vector kinds, and places captionless
  block vectors on the next fixture page. The complete writer emits one shared
  Form, four page-level Figure/Formula MCRs, property-only ActualText/Lang
  Spans, and a following equation-number Span using its original shaping
  receipt and an embedded TrueType subset with Type0/CID/ToUnicode resources.
  The Rust and Python independent validators re-derive the object, ParentTree,
  outline, marked-content, extraction, and Form-isolation closure. Separate
  synthetic cases check IDTree closure, Unicode equation numbers, and
  ActualText-only font extraction. All three final writer observations
  (tagged PDF, book navigation, and SafeVector) close over the same PDF hash.
- The `MI4-V18` integration evidence runner composes those production
  owners in strict receipt order, publishes the complete generated artifact
  set only below `workspace/target/machine-e2e/precomposed-vector/`, reruns the
  legacy SafeVector/native-math/navigation/tagged `/1` owners, and separately
  generates the one-Form/ten-`Do` proof plus a two-logical-ID, two-provenance,
  one-Form alias proof. It also stable-reads and admits all 13 logical corpus
  resources through the Safe-SVG 2 parser, closes the two equal hashes to 12
  content candidates, and publishes that canonical receipt as
  `corpus-admission.json`. The assertion-only all-category projection emits
  `corpus-display.json` and an eight-page `corpus-output.pdf`: all 33 ledger
  occurrences use the admitted vector paths as 12 shared Form XObjects and 33
  `Do` operations, preserve Formula/Figure ActualText and language in document
  order, and never rasterize. It supplements rather than replaces the
  fully-tagged four-kind production PDF and its structure tree. The runner also
  publishes `effective-document-package.json`, the exact canonical checked
  package consumed by that production path; its hash is joined to the
  SafeVector, math-vector, tagged-PDF, and book-navigation package identities.
  The independent verifier joins these generated PDF/Display/package facts to
  this directory's all-category Japanese corpus ledgers and fails on a missing,
  extra, stale, rasterized, or inconsistent artifact. Host evidence binds the
  exact package, source set, TeX set, fragment-text set, all four ledgers,
  expectation, verifier, Cargo lockfile, Git revision, and complete worktree
  snapshot. Generated PDFs and host evidence are never checked in here.
  `MI4-V19` adds `tagged-pdf-expectation.json` for the independent `/2`
  structure validator, bringing the current generated directory to 21 indexed
  artifacts plus `artifact-index.json` (22 files total). It also adds the
  pinned external renderer/extractor/veraPDF gate, exact Matterhorn 1.02 `/2`
  assessment, and complete production resource-set `/2` proof without
  publishing the profile itself; MI4-13 later performed that atomic publication.

`resources.tsv` is joined to `cases.tsv` by `image_id`; the repeated
`expected_sha256` must match on both sides. An image ID is a dense logical ID,
while `uri` is derived from the full SVG SHA-256. Distinct IDs may therefore
share one URI and byte sequence. `engine_id`, `engine_version`, and
`rules_version` are producer assertions bound through that join; the corpus
gate records them but does not execute the named engine.

All metric and spacing fields use the package coordinate unit
`pdf_point_1_65536`. `advance`, `ascent`, `descent`, `origin_x`, `baseline`,
`viewport_width`, and `viewport_height` are required for `math_vector`,
`inline_vector`, and `math_vector_block`. `vector_figure` instead carries only
the two viewport fields. Full metric rows must satisfy:

```text
advance > 0
ascent > 0
descent >= 0
viewport_width > 0
viewport_height > 0
0 <= baseline <= viewport_height
ascent >= baseline
descent >= viewport_height - baseline
```

`origin_x + viewport_width` must also fit in a signed 64-bit integer, and the
viewport must be a single-scale round-half-to-even result from the SVG root
geometry. `baseline` is the downward distance from the viewport top to the
formula baseline; `origin_x` is the signed distance from the inline pen origin
to the viewport left. The intended placement equations are therefore:

```text
viewport_left = pen_x + origin_x
viewport_top = line_baseline_y - baseline
line_baseline_y = viewport_top + baseline
```

`advance`, rather than the viewport bounding width, is the atomic inline
occupancy used by line breaking. Inline spacing is nonnegative. Block spacing
is deliberately `-` because its owner is the future typed style contract. An
equation number and its positive `minimum_gap` are either both present on
`math_vector_block` or both `-`.

For math kinds, `source_tex_path` is required and `actual_text = -` means the
authored `alt` is the resolved fallback. Generic vector kinds require both
`source_tex_path` and `actual_text` to be `-`. `language = inherit` records
document-language inheritance; every other value is an explicit override.
The ordered `cases` list in `fragments.tsv` assigns a placement ordinal to
each marker, including repeated markers. Its optional
`inline_remaining_width`, `block_frame_width`, `block_remaining_height`, and
`next_empty_frame_height` fields use the same fixed-point unit. The line-end
fixture binds remaining width to `advance + spacing.before` and excludes the
positive `spacing.after` at the selected line end. The page-end fixture binds
a 450pt block to a 480pt frame, a 16pt current-page remainder, and a 600pt
empty next frame.

All ledgers use UTF-8, LF, a final LF, one exact header, tab-separated fields,
and canonical row order. Comma-separated lists are nonempty, unique, and
UTF-8 byte sorted unless occurrence order is explicitly part of the field.
`-` means null or inapplicable according to the case kind. Every referenced
file is a contained regular file below this directory. TeX files include their
final LF as part of the exact opaque source bytes.

The SVG files are already lowered by the producer boundary: glyph-like
geometry is expanded into the Safe-SVG 2 path/shape subset. They intentionally
contain no `use`, text/font nodes, CSS, script, animation, image, or external
reference. Typaxis must validate and place these bytes; it must not add a VMB
preprocessor or fall back to native math, PNG, or omitted content.

Run the integration evidence and independent verification from the repository
root with:

```sh
cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli \
  machine_precomposed_vector_closes_production_pipeline_and_legacy_paths --locked
python3 tools/verify_precomposed_vector.py \
  workspace/target/machine-e2e/precomposed-vector --repository .
```

`verify_precomposed_vector.py --emit-host-evidence PATH` writes one canonical,
Schema-validated `/2` host record with atomic replacement. The writer requires
`--require-external-tools`, the generated production readiness directory, the
Typaxis binary, and pinned tool versions from
`../external-tool-policy.json`; missing tools and veraPDF warnings fail. Run:

```sh
TYPAXIS_VERAPDF=/path/to/verapdf-1.30.2/verapdf \
cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli \
  machine_precomposed_vector_external --locked -- --ignored
```

The ignored test publishes proof/evidence below `target/machine-e2e/`. Its
aggregate mode uses
`--require-host-evidence DIRECTORY --required-host macos --required-host linux`
and accepts only current-revision, successful records with one identical
artifact-set and production receipt. Aggregation recomputes current source,
verifier, fixture, 73 resources, capability, Matterhorn, and tool-policy
identities, so two mutually consistent but stale records are still rejected.
The distinct path/locale/timezone/filesystem-order gate is owned by
`tools/verify_reproducibility.py --private-staging-test precomposed-vector`;
this is a repository verification-tool option, never a public `typaxis` CLI
option.

The PDF contribution tests use `x-plus-y.svg` for exact public black
`currentColor`, `fraction-equality.svg` for distinct fill/stroke alpha,
`matrix.svg` for local clipping and fixed RGB paint, and the admitted but
unselected `similar.svg` for zero-use audit closure. Form content is inspected
as path operators with an intrinsic `/BBox`, explicit Form-local ExtGState,
and no raster or semantic marked content. Page usages are inspected in the
exact `q`, nonstroking/stroking RGB, top-left matrix, `Do`, `Q` order under one
existing page-root Y flip.

Both selected-layout trace files are canonical JCS and are validated against
the current 1.4 layout-trace schema. The block trace deliberately starts with a
partly consumed page: the Figure and its kept caption fit there, while the
numbered math block moves intact to the next page. Its `pagination_bounds`,
`paint_bounds`, and `structure_bounds` are identical; the producer viewport
remains unchanged; and the `formula` child precedes the independent
`equation_number` child. The Display trace is canonical JCS and is validated
against the current 1.4 display-list schema. It contains no resource URI, raw
SVG, source TeX, PDF object/name, or MCID; those facts remain reachable only
through sealed binding and selected-placement fingerprints.
