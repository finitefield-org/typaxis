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
mode. The public profiles are `typaxis.machine-pdf/paragraph-1` and
`typaxis.machine-pdf/basic-document-1`. `paragraph-1` remains the default when
`--profile` is omitted; Typaxis never infers a profile from package contents.

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

Source and resource roots are intentionally separate. Both public profiles accept exactly one
source declaration, `SourceId` 0, with entry-only closure. Its URI is resolved
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

`typaxis.machine-pdf/basic-document-1` requires a raw
`typaxis.contract/1.2` package and accepts the exact additional descriptor set:

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
tagged PDF, math, and every M3-or-later feature. Raw 1.0/1.1 packages selected
with this profile fail at `/contract`; Typaxis does not synthesize 1.2 values.
The combined fixture renders two pages and Poppler-normalized text exactly
`Basic document internal external First item Second entry PNG caption`.

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

Machine diagnostics are canonical JSON under `typaxis.contract/1.2`. A primary
location is either `byte` offset, package JSON Pointer, source byte location, or
global/null. The main public code families are:

| Codes | Meaning |
| --- | --- |
| `P1100`–`P1103` | package envelope, JSON grammar/type, or contract rejection |
| `P1110`–`P1112` | source closure, safe path, or source/TextMap identity rejection |
| `L5100`, `L5101`, `L5110` | unsupported document/style capability or layout limit |
| `R7100`, `R7110`, `R7111` | unsupported resource, image pixels, or decoded-image bytes |
| `T2100`, `T2101`, `G6100` | text-buffer/text-total or PDF object limit |
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
`basic-document-1`, it additionally binds the canonical all-flow registry hash.
Receipt/profile/package/session substitution is an internal `I9190` failure,
not a producer-recoverable fallback. A built manifest binds the file/stdout sink, PDF bytes,
SHA-256, page count, and object count. A failed manifest has `output: null` and
contains only facts reached before failure.

Diagnostics or manifest publication can itself fail with exit 3. Already
visible artifacts are reported by the typed publication outcome and are not
described as rolled back. `--force` permits atomic target replacement but never
permits a target to alias the package, source, config, resource candidate, or
another output/sidecar.

## Round trip and reproducibility

For supported reference TSF, this round trip is defined:

```text
typaxis dump-ast input.tsf --format json > document-package.json
typaxis build-package document-package.json -o output.pdf
```

`dump-ast` emits contract 1.2. Because its reference-TSF subset is also within
`paragraph-1`, the default profile remains valid for this round trip.

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
