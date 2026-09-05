# Machine input CLI producer guide

This document is the normative producer guide for the public machine-input
surface. The key words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are to be
interpreted as requirements on a producer. The closed capability descriptor is
available from `typaxis capabilities --format json`; producers MUST inspect that
descriptor instead of assuming support for later profiles or richer document
features.

## Public commands

```text
typaxis build-package PACKAGE -o OUTPUT [OPTIONS]
typaxis check-package PACKAGE [OPTIONS]
typaxis capabilities --format json
```

The complete grammar is emitted by `typaxis help build-package`,
`typaxis help check-package`, and `typaxis help capabilities`. `build` continues
to accept bounded reference TSF; it never sniffs JSON or switches to machine
mode. The public profiles are `typaxis.machine-pdf/basic-document-1`,
`typaxis.machine-pdf/columns-1`,
`typaxis.machine-pdf/float-1`, `typaxis.machine-pdf/footnote-1`,
`typaxis.machine-pdf/header-footer-1`, `typaxis.machine-pdf/paragraph-1`,
`typaxis.machine-pdf/production-book-1`, and
`typaxis.machine-pdf/table-1`.
`paragraph-1` remains the default when `--profile` is omitted; Typaxis never
infers a profile from package contents.

## Package directory and roots

A minimal producer-owned directory has this shape:

```text
job/
├── document-package.json
├── sources/
│   └── book.json
└── fonts/
    ├── body.ttf
    └── collection.ttc
```

`PACKAGE` is a host path. Without `--package-root`, its lexical parent is the
package root. With `--package-root DIR`, `PACKAGE` MUST be lexically contained
beneath `DIR` before any package read. The package and its companion source are
then opened relative to one contained root using no-follow, stable-read host
admission. Symlinks, non-regular files, root escapes, read mutation, and declared
length/hash mismatches fail closed.

Source and resource roots are intentionally separate. All eight public profiles
accept exactly one source declaration, `SourceId` 0, with entry-only closure. Its URI is resolved
only beneath the package root. Font resources are resolved only from explicit
`--resource-root DIR` values and configured resource roots; a package root is
not implicitly a resource root. Canonical artifacts and manifests never contain
absolute host paths.

## Supported profiles

### `paragraph-1`

`typaxis.machine-pdf/paragraph-1` accepts the exact descriptor set:

- paragraph and heading blocks;
- text, anchor, page reference, soft break, and hard break inlines;
- paragraph/heading selectors with `font_family`, `font_size`, `line_height`,
  and `page` declarations;
- one default master, the `auto` page value, and entry-only source closure;
- TrueType `glyf` faces in standalone sfnt or TTC containers;
- named destinations and normalized text extraction.

It does not accept lists, figures, images, tables, footnotes, page breaks,
columns, floats, math, vector content, link annotations, outlines, tagged PDF,
or heading semantics. The profile ID is immutable: later support will use a new
profile and contract rather than widening `paragraph-1`.

Generated page-reference labels are painted as PDF Artifact content. They stay
in the layout trace when `--trace-text` is selected but are excluded from
normalized document text extraction. The combined fixture therefore extracts
exactly `Typaxis machine input`.

### `basic-document-1`

`typaxis.machine-pdf/basic-document-1` accepts a raw
`typaxis.contract/1.2` package or its exact neutral 1.3 encoding and accepts the
exact additional descriptor set:

- list, figure, and forced `page_break` blocks, including independent list-item
  and figure-caption flows;
- non-nested painted internal/external links in addition to the paragraph
  inline set;
- paragraph, heading, list, figure, and page-break selectors;
- `space_before`, `space_after`, `start_indent`, `end_indent`, `text_align`,
  `width`, `keep_with_next`, and `keep_caption` in addition to the paragraph
  properties;
- ordered/unordered checked list markers, non-floating decoder-attested PNG
  figures, and internal named destinations or validated
  `http`/`https`/`mailto`/`tel` link annotations.

It still rejects tables, footnotes, emphasis/strong, nested or unpainted links,
named-page/master behavior, JPEG/SVG/vector/float content, OTF/CFF, outlines,
tagged PDF, math, and advanced pagination. Raw 1.0/1.1 packages selected with
this profile fail at `/contract`; Typaxis does not synthesize 1.2 values.
The combined fixture renders two pages and Poppler-normalized text exactly
`Basic document internal external First item Second entry PNG caption`.

### `table-1`

`typaxis.machine-pdf/table-1` accepts raw `typaxis.contract/1.2` or its exact
neutral 1.3 encoding,
inherits the complete `basic-document-1` domain, and adds direct document-body
tables. It must be selected explicitly. `paragraph-1` stays the default, and
both older profiles continue to reject every table.

The table subset is closed: one or more fixed/fraction columns, `head` followed
by `body`, dense non-overlapping colspan/rowspan coverage, and cell flows made
only from paragraphs containing text, soft breaks, or hard breaks. Columns are
resolved in integer `pdf_point_1_65536` units; fractional rounding uses
ties-to-even and assigns the signed residual only to the last fraction column.
The complete header group repeats on every continuation page and remains bound
to its original row, cell flow, and dense repetition index.

Table selectors accept only `page = auto`, `space_before`, `space_after`,
`start_indent`, `end_indent`, and `keep_with_next`. Cell paragraphs use the
existing paragraph style rules. Border, background, padding, vertical
alignment, border spacing, and authored split/repeat controls are not contract
1.2 fields. The fixed visual policy is no border, transparent background, zero
padding/spacing, and block-start content; the table contributes zero path or
decoration operations. Unsupported placement/content is `L5100`, an
inapplicable known property is `L5101`, and an invented raw declaration is
`P1102`.

Successful table traces and built manifests carry identical `table_layouts`
facts for resolved columns, grid/cell FlowIds, selected row pieces, rowspan
continuations, header occurrences, and the selected-layout hash. Display and
the frozen PDF graph bind the exact retained cell glyph commands and reject a
missing, extra, relocated, repeated-as-the-wrong-header, or decorated table
before publication. The combined fixture renders three pages and extracts
exactly `Basic document internal external First item Second entry PNG caption
Header A Header B alpha beta Header A delta Header B gamma`.

### `footnote-1`

`typaxis.machine-pdf/footnote-1` accepts raw contract 1.2 or its exact neutral
1.3 encoding, inherits the
complete `basic-document-1` domain, and adds body/list-item/caption footnote
references plus Document-owned paragraph/heading definitions. Tables and
nested footnotes remain rejected. Definitions use the existing M2 inline/style
subset, including anchors, page references, breaks, and non-nested links; every
definition must be referenced and text-producing.

Marker numbers are one-based canonical FootnoteId catalog ordinals, while page
assignment and paint use selected first-reference order. A repeated reference
repaints only its marker. The fixed `allow` split policy carries unfinished
definition cursors independently of the body cursor, including onto carry-only
pages. Each nonempty footnote page has an exact reservation and one fixed black
0.5 pt full-width separator; empty footnote pages reserve and paint nothing.
Successful trace and manifest artifacts contain identical `footnote_layout`
facts binding body/selected/paint hashes, evaluation counts, ordered IDs,
FootnoteFlowIds, fragment cursors, reservations, and carry edges. The combined
fixture renders three pages and extracts exactly `Basic document internal
external First item Second entry Z first Z second A note A tail PNG caption Z
third Z fourth Z fifth`.

### `columns-1`

`typaxis.machine-pdf/columns-1` requires raw contract 1.3 and inherits the
complete `basic-document-1` content, style, resource, and PDF domain. Every 1.3
page-master set explicitly declares `writing_mode = horizontal-tb` and
`page_progression = ltr`; every master declares trim, nullable region content,
and nullable column layout; every Figure declares `placement`.

This profile accepts one default master, full-media trim, no auxiliary region
or footnote content, no floating Figure, and either one body column (null
layout) or 2..65,535 left-to-right sequential columns. A non-null layout uses
`balance = last_page`. Only the final nonempty page is balanced, and candidate
targets strictly increase. The inclusive `max_column_balance_candidates`
maximum may win; `G6003` is emitted before max+1 or on oscillation. Trace and
manifest carry identical selected column frames and balance facts.

### `float-1`

`typaxis.machine-pdf/float-1` requires raw contract 1.3 and inherits the same
complete basic domain. It uses one full-media default master and one or more
left-to-right sequential columns with `balance = none`. A Figure with
`placement = float` must be a direct document-body child; its image and caption
are one unsplittable unit.

Floats are evaluated FIFO in `here`, `top`, `bottom`, `next_page` order without
side wrapping, bypass, scaling, clipping, or block fallback. Queue length and
per-float page carry are inclusively bounded by `max_float_queue` and
`max_float_carry_pages`; `G6004` is emitted before max+1. Selected placements,
queue transitions, carries, Display commands, and PDF object usage share one
closure.

### `header-footer-1`

`typaxis.machine-pdf/header-footer-1` requires raw contract 1.3 and inherits
the complete basic domain. It accepts a checked custom trim and either one
default master or exactly the canonical first/left/right set. The latter has a
right default, a dense first rule followed by an even-page left rule, and no
named pages. Columns, floats, and footnotes are rejected.

Each non-null header/footer rectangle has matching static region content made
only from paragraph/heading blocks and text/soft-break/hard-break inlines. Its
MasterId-bound FlowId restarts at source start for each selected repetition and
must reach terminal in that one region frame. Selected pages serialize
explicit MediaBox, CropBox, and TrimBox facts; trace and manifest bind the same
header/body/footer frame and repetition order used by Display and PDF.

### `production-book-1`

`typaxis.machine-pdf/production-book-1` requires raw contract 1.4 and explicit
profile selection. It is the only public profile that accepts the M4 semantic,
media, navigation, tagged-PDF, and producer-composed vector domains. Contract
1.4 selected with an old profile, or selected through the default
`paragraph-1`, fails with `P1103` before resource open. Contracts 1.0 through
1.3 selected with `production-book-1` fail with `L5100` before resource open;
their failed 1.4 manifest envelope may preserve provenance-bound legacy media
declarations but never treats them as attested.

The closed profile adds `semantic_container`, native `inline_math` and
`display_math`, producer-composed `inline_vector`, `math_vector`,
`vector_figure`, and `math_vector_block`, plus the existing list, table,
footnote, Figure/caption, link, heading, emphasis, and strong domains. It
requires explicit metadata, document language, and outline records and emits a
tagged PDF with source-bound alternatives, ActualText, language, destinations,
links, and structure relations. The complete production fixture is the
normative positive coverage input; a partial feature-staging fixture is not a
substitute for that profile closure.

Every contract-1.4 resource declaration has a required typed `media_type`.
The exact production set is:

- images: `png`, `svg-safe-1`, `svg-safe-2`, `jpeg-baseline`;
- fonts: `sfnt-truetype-glyf`, `ttc-truetype-glyf`, `sfnt-cff1`;
- resource-set identity: `typaxis.production-book-resource-set/2`.

Safe SVG is parsed as a closed, bounded vector language and converted to PDF
Form XObjects; unsupported elements, attributes, references, script,
animation, CSS, text/font/image primitives, and network/filesystem access are
terminal `R7100`. `svg-safe-2` additionally permits the exact currentColor and
per-paint opacity subset. Inline vector line width uses `advance`; line height
uses `ascent` and `descent`; the producer baseline is aligned to the text
baseline; and the item is indivisible. Block vectors preserve aspect ratio,
alignment, spacing, keep behavior, independent equation-number placement, and
atomic pagination. Repeated verified content keys reuse the same vector Form
XObject rather than rasterizing or duplicating it.

Production builds require `--no-compress`; omitting it is a usage failure
before PDF construction. The successful manifest uses the 1.4 Schema and binds
declaration/attestation equality, SafeVector `/2`, math-vector `/1`,
book-navigation `/2`, tagged-PDF `/2`, selected layout, PDF hash, and all
resource/font transformations. The seven earlier profiles retain their frozen
1.3 artifact encoders and accepted raw-contract sets.

## Checking and building

Example validation:

```text
typaxis check-package job/document-package.json \
  --package-root job \
  --profile typaxis.machine-pdf/paragraph-1 \
  --resource-root job \
  --emit-diagnostics check-diagnostics.json
```

For a contract 1.2 basic-document package, select the profile explicitly:

```text
typaxis check-package job/document-package.json \
  --package-root job \
  --profile typaxis.machine-pdf/basic-document-1 \
  --resource-root job \
  --emit-diagnostics check-diagnostics.json
```

For a contract 1.2 table package, select the table profile explicitly:

```text
typaxis check-package job/document-package.json \
  --package-root job \
  --profile typaxis.machine-pdf/table-1 \
  --resource-root job \
  --emit-diagnostics check-diagnostics.json
```

For a contract 1.2 footnote package, select the footnote profile explicitly:

```text
typaxis check-package job/document-package.json \
  --package-root job \
  --profile typaxis.machine-pdf/footnote-1 \
  --resource-root job \
  --emit-diagnostics check-diagnostics.json
```

For a contract 1.3 advanced package, select its matching immutable profile;
the default never infers advanced behavior. For example:

```text
typaxis check-package job/document-package.json \
  --package-root job \
  --profile typaxis.machine-pdf/columns-1 \
  --resource-root job \
  --max-column-balance-candidates 2 \
  --emit-diagnostics check-diagnostics.json
```

For a contract 1.4 production package, select the production profile
explicitly:

```text
typaxis check-package job/document-package.json \
  --package-root job \
  --profile typaxis.machine-pdf/production-book-1 \
  --resource-root job \
  --emit-diagnostics check-diagnostics.json
```

A successful check guarantees stable package/source admission, strict bounded
JSON decoding, semantic source/TextMap validation, profile preflight, resource
metadata admission, computed styles, and font-family resolution. It does not
guarantee final glyph coverage, pagination, PDF serialization, or publication.
Build-only options such as `-o`, `--strict`, `--no-compress`, `--trace`,
`--trace-text`, `--emit-build-manifest`, and `--force` are usage errors for
`check-package`; they are never silently ignored.

Example build with every sidecar:

```text
typaxis build-package job/document-package.json \
  -o output.pdf \
  --package-root job \
  --profile typaxis.machine-pdf/basic-document-1 \
  --resource-root job \
  --trace trace.json --trace-text \
  --emit-build-manifest manifest.json \
  --emit-diagnostics diagnostics.json
```

The corresponding production build adds the mandatory `--no-compress` flag:

```text
typaxis build-package job/document-package.json \
  -o output.pdf \
  --package-root job \
  --profile typaxis.machine-pdf/production-book-1 \
  --resource-root job \
  --no-compress \
  --trace trace.json --trace-text \
  --emit-build-manifest manifest.json \
  --emit-diagnostics diagnostics.json
```

`--trace-text` requires `--trace`. A package containing generated text, such as
a page reference, MUST use `--trace-text` when requesting a complete trace.
`OUTPUT=-` alone selects PDF stdout; `./-` is a normal filename. Each file is
published atomically, but the set of PDF and sidecars is not a multi-file
transaction. Success publication order is trace, PDF, diagnostics, then built
manifest. Processing failure never publishes a PDF; it attempts diagnostics and
then a failed manifest.

`capabilities --format json` writes canonical JSON directly from the compiled
descriptor. It does not read project configuration, environment overrides,
filesystem inputs, or the ambient locale. Missing `--format`, formats other
than `json`, positional arguments, and duplicate format options are usage
errors.

## Diagnostics and locations

Machine diagnostics use profile-dispatched canonical JSON: old-profile package
commands retain the frozen 1.3 envelope, while current source export and
`production-book-1` use 1.4. A primary location is either `byte` offset,
package JSON Pointer, source byte location, or global/null. The main public code
families are:

| Codes | Meaning |
| --- | --- |
| `P1100`–`P1103` | package envelope, JSON grammar/type, or contract rejection |
| `P1110`–`P1112` | source closure, safe path, or source/TextMap identity rejection |
| `L5100`, `L5101`, `L5110`, `L5111` | unsupported document/style capability, selected-item limit, or math/vector layout work limit |
| `R7100`, `R7110`, `R7111` | unsupported resource, image pixels, or decoded-image bytes |
| `R7120`–`R7122` | SafeVector node, path-segment, or nesting limit |
| `R7130`–`R7135` | CFF table, glyph, subroutine, operation, outline, or subset-byte limit |
| `T2100`, `T2101`, `G6100` | text-buffer/text-total or PDF object limit |
| `G6003`, `G6004` | column-balance candidate or float queue/carry bound |
| `I9100`, `I9101`, `I9102` | package bytes, JSON depth, or host work limit |
| `I9110`–`I9113` | host unavailable, unsafe package/source open, or stable-read/alias failure |
| `I9190` | internal receipt/phase contradiction; producers MUST NOT retry as another input mode |

The command-wide diagnostic budget is 256 records. If more failures are
observed, fatal diagnostics are retained and a canonical omission note records
the truncation.

Exit codes are stable classes:

| Exit | Class |
| ---: | --- |
| 0 | success |
| 1 | input, profile, layout, or strict-fallback diagnostic |
| 2 | command grammar, option, path-containment, or unknown-profile usage error |
| 3 | host I/O, contained-open availability, or publication failure |
| 4 | internal invariant failure |
| 5 | configured or fixed resource limit |

## Manifest facts

A requested terminal manifest always has `input_profile` set to the resolved
machine profile. `package_input` progresses from raw byte/hash facts to known
contract/canonical hash facts after decode. On machine success it also contains
the profile preflight receipt hash. `inputs` contains only successfully
admitted companion sources; the package is not duplicated there. `fonts` and
`images` contain only reached resource-admission facts. `layout` appears only
after layout selection and repeats the matching profile receipt hash. For
`basic-document-1`, `footnote-1`, and `table-1`, it additionally binds the
canonical all-flow registry hash. Successful `table-1` artifacts contain the
same canonical `table_layouts` projection, while successful `footnote-1`
artifacts contain the same canonical `footnote_layout` projection. Other
old profiles omit those conditional members so their artifact bytes remain
frozen. A built advanced profile requires a byte-identical
`advanced_pagination` projection in trace and manifest; it binds profile,
complete flow registry, selected pages/frames/repetitions/balance/float state,
and Display/PDF paint closure. Old profiles forbid that member.
Built `production-book-1` manifests additionally require the complete
production media declaration/attestation records and nonnull fingerprinted
SafeVector `/2`, math-vector `/1`, book-navigation `/2`, and tagged-PDF `/2`
pairs, even when an individual permitted kind has zero selected uses. A failed
production manifest admits only facts reached before the terminal error;
legacy declarations and null attestations are restricted to the sealed
pre-resource compatibility rejection.
Receipt/profile/package/session substitution is an internal `I9190` failure,
not a producer-recoverable fallback. A built manifest binds the file/stdout sink, PDF bytes,
SHA-256, page count, and object count. A failed manifest has `output: null` and
contains only facts reached before failure.

Diagnostics or manifest publication can itself fail with exit 3. Already
visible artifacts are reported by the typed publication outcome and are not
described as rolled back. `--force` permits atomic target replacement but never
permits a target to alias the package, source, config, resource candidate, or
another output/sidecar.

## Source export and reproducibility

`dump-ast` exports current contract 1.4. Re-input never infers a profile: when
the exported document satisfies the complete production profile, use the
explicit production selection and build policy:

```text
typaxis dump-ast input.tsf --format json > document-package.json
typaxis build-package document-package.json -o output.pdf \
  --profile typaxis.machine-pdf/production-book-1 \
  --no-compress
```

Before writing stdout, `dump-ast` performs stable admission of every declared
resource and derives each 1.4 `media_type` from decoder-issued attestation.
Missing, malformed, mismatched, or concurrently changed resource bytes fail
without partial JSON. URI suffixes and source strings are never format
authority. The exported package is not a capability bypass: omission still
selects frozen `paragraph-1` and rejects raw 1.4, while an incomplete
production document still fails the normal closed-domain preflight.

Whitespace and object-member order may change raw JSON bytes and the raw hash;
they do not change canonical JCS, the canonical package hash, or the typed
DocumentFingerprint. Semantic changes do. Producers MUST recompute declared
source/resource byte lengths and SHA-256 values when those bytes change.

The public basic-document release gate is:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m2-basic.json \
  --runs 2 \
  --require-external-tools
```

The public table release gate is:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m3-table.json \
  --runs 2 \
  --require-external-tools
```

The public footnote release gate is:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m3-footnote.json \
  --runs 2 \
  --require-external-tools
```

The complete public M3 gate runs the table, footnote, columns, float, and
header/footer combined fixtures together:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m3-all.json \
  --runs 2 \
  --require-external-tools
```

The complete public M4 gate runs the combined production fixture and frozen
old-contract rejection together:

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --matrix samples/machine-package/matrices/m4-production.json \
  --runs 2 \
  --require-external-tools
```

The paragraph compatibility gate may use
`--fixture samples/machine-package/profiles/paragraph-1/combined/expected.json`.
The verifier clean-builds the current worktree, validates all JSON artifacts,
checks the exact public profile/contract/default closure and profile receipts, compares
PDF/trace/manifest/diagnostics/capabilities bytes across runs and differently
named source snapshots, and requires MuPDF raster plus Poppler page/text
observations. It atomically writes canonical host evidence beneath
`target/machine-e2e/host-evidence/`. Missing external tools are a failed gate,
not a successful skip. Release support additionally requires current-revision
Linux and macOS evidence to pass the aggregation command documented in the
sample README. Each host command is run explicitly on a managed host; this
repository does not use GitHub Actions or GitHub workflow files.
