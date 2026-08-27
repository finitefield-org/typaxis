# Schema validation

## Delivery status and trust boundary

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| DocumentPackage portable shape | Yes, current 1.2 plus frozen 1.0/1.1 input | Yes: independent Schema/offline semantic validation | Yes, package commands | Yes |
| `dump-ast` DocumentPackage export | Yes, current 1.2 | Yes, shared converter/encoder | Yes, supported package round trip | Yes |
| sealed package/source admission | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, closed contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/basic-document-1` | Yes, ADR-0028 | Yes: full profile, receipt, multi-flow, style/list/break/figure/link closure | Yes, combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/table-1` | Yes, ADR-0029 | Yes: grid/cell-flow, fragmentation/header, Display/PDF, trace/manifest closure | Yes, table-only and combined PDF/sidecars | Yes, MI3-04 gate |
| `typaxis.machine-pdf/footnote-1` | Yes, ADR-0030 | Yes: discovery/reflow, dedicated carry, Display/PDF, trace/manifest closure | Yes, zero and combined PDF/sidecars | Yes, MI3-07 gate |
| contract 1.1 Schema registry | Yes | Yes: frozen eleven-schema compatibility registry | Compatibility input only | Frozen |
| contract 1.2 Schema registry | Yes, ADR-0028/ADR-0029/ADR-0030 | Yes: current aliases plus complete independent nineteen-schema versioned registry | Yes | Yes |
| contract 1.3 advanced-pagination registry | Yes, ADR-0031 target | Partial: private three-schema staging registry with header/footer and columns goldens; float/public registry pending | No; current aliases remain 1.2 | No, MI3-12 gate |

The offline validator proves portable Schema and semantic conformance only. It
does not issue in-process admission or validation receipts. The `typaxis build`
command intentionally does not accept `document-package.schema.json` instances;
the separate public `build-package` and `check-package` commands perform sealed
in-process admission. Their normative contract is [docs/26](../docs/26-machine-input-cli.md),
and remaining release evidence is tracked in
[docs/25](../docs/25-machine-input-pdf-improvements-todo.md).

`WireDocumentPackage` is intentionally caller-constructible and untrusted.
Portable validation cannot manufacture the decoder-issued
`DecodedDocumentPackage`, bind raw and canonical package hashes to one host
session, admit the exact companion source bytes, or issue a
`ValidatedMachinePackage`/capability receipt. The internal trusted path calls the
strict decoder only on stable bytes owned by machine admission and lets sealed
`typaxis-syntax` perform source/TextMap/domain validation. A validator success,
`dump-ast` JSON, or matching hashes therefore never substitutes for an
in-process receipt.

[ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md) and
[ADR-0028](../adr/ADR-0028-basic-document-profile.md) define the input/profile migrations,
and [ADR-0029](../adr/ADR-0029-table-profile.md) defines the table profile on
that unchanged wire. [ADR-0030](../adr/ADR-0030-footnote-profile.md) defines
the public footnote profile. `schemas/1.0/` contains the frozen seven-schema
registry and `schemas/1.1/` contains the frozen former-current eleven-schema
registry. Top-level `schemas/*.schema.json` files are current 1.2 aliases,
including capability, fixture/matrix, and machine host-evidence Schemas. Current
generators emit 1.2, while the typed DocumentPackage parser recognizes 1.0,
1.1, and 1.2. The public `build-package`, `check-package`, and capability CLI
surface supports all four immutable profiles.

`schemas/1.2/` is the complete current versioned registry. MI2-02 added
canonical multi-flow trace/manifest projections; MI2-03 added the additive
DocumentPackage property tags and the selected typed-style manifest fact;
MI2-04 added selected list-flow, marker-usage, fragment, geometry, and PDF hash
closure; MI2-05 added forced-boundary trace/manifest cursor consumption,
blank-page, and PDF page-tree closure; MI2-06 added decoder-attested PNG,
figure/caption placement, DrawImage, and serialized image-XObject closure;
MI2-07 added package-anchor/normalized-URI targets, logical shaping-cluster
ranges, selected page/line rectangles, named destinations, and serialized link
annotation closure. MI2-08 completed the remaining general artifact Schemas,
froze 1.1, and switched all top-level aliases atomically. MI3-04 added
`machine-table-manifest.schema.json` and the conditional `table_layouts`
projection to trace/build-manifest Schemas: it is required for a built
`table-1` artifact and forbidden for older profiles, preserving their bytes.
MI3-07 added `machine-footnote-manifest.schema.json` and the conditional
`footnote_layout` projection: it is required for a built `footnote-1` artifact
and forbidden for the other profiles.

Schema `$id` values under `https://schemas.typaxis.invalid/1.0/`,
`https://schemas.typaxis.invalid/1.1/`, and
`https://schemas.typaxis.invalid/1.2/` are logical, offline
identifiers. They are not fetch URLs. A validator must build independent
registries for each version and register every `*.schema.json` file by its
`$id` before resolving relative `$ref` values.

Run the bundled offline validator from the repository root:

```text
python3 schemas/validate.py
```

It requires Python 3.11 or later and `jsonschema` 4.18 or later. The validator:

- meta-validates the frozen 1.0/1.1 and current/versioned 1.2 Draft
  2020-12 registries and resolves every registered `$ref` without
  cross-registering versions;
- proves that the canonical 1.0 compatibility fixture is accepted by the
  frozen DocumentPackage Schema but not the current registry, retains its 1.0
  contract member in its JCS hash, and that a 1.0 consumer rejects the additive
  1.1 config, diagnostics, and manifest shapes;
- validates all minimal JSON and TOML fixtures;
- checks every invalid fixture against its authoritative `schema_rejects` value
  and requires its semantic conformance check to emit exactly the indexed
  `rule_id`;
- checks URI-link schemes against the effective configuration allowlist and
  checks URI UTF-8 byte length against effective `max_uri_bytes`;
- checks canonical UTF-8-byte ordering for effective resource roots, URI
  schemes, and block classes, plus style selector/class, list-start, dense
  source-order, unique-style-ID, and known acyclic `extends` relationships;
- checks DocumentPackage source/text/resource/anchor/footnote ID uniqueness,
  known references, owner bounds, text UTF-8 boundaries, page-master integrity,
  table grid coverage, and the typed `page` style value;
- checks dense trace/display indexes, exact checked layout costs, first-terminal
  convergence, DisplayDocument text-buffer span/cluster coverage, destination bounds,
  graphics-state balance, and nonempty materialized page lists;
- checks that basic-document FlowIds and owner-local positions are dense, every flow
  has exactly one final terminal, child edges match canonical parents, and the
  versioned basic-document manifest covers the exact same terminal/hash set;
- validates the eight exact 1.2 block-style tags and min/max/max+1 boundaries,
  rejects unknown/wrong-tag values, verifies every advertised property has a
  layout/Display/PDF/manifest consumer, and checks the page-split/PDF fixture;
- validates ordered/unordered marker derivation, item/list FlowId ownership,
  nested child-flow frames, widest-column end alignment, marker/first-line
  fragment keep, dense pages/fragments, exact marker keys, the canonical runner
  golden, and wrong-marker/orphan semantic negatives for MI2-04;
- validates MI2-05's exact one-step forced-break cursor consumption, `N + 1`
  page relation, leading/consecutive/trailing blank pages, canonical trace and
  manifest runner goldens, exact/max+1 page limit, and stale-cursor semantic negative;
- validates MI2-06's opaque-suffix PNG admission hash and dimensions, ties-to-even
  aspect placement, caption FlowId keep/split facts, exact one-DrawImage and
  logical-image/XObject closure, canonical deterministic runner golden, and
  missing/extra/wrong-XObject semantic negatives;
- validates MI2-07's package-bound internal anchor and scheme-normalized external
  URI, nonempty contiguous logical cluster ranges, canonical page/line rectangle
  order and bounds, named-destination ownership, per-page annotation counts,
  exact PDF annotation-object closure, deterministic runner golden, and
  missing/extra/wrong-page/wrong-target/rectangle semantic negatives;
- validates MI3-04's table-only and complete-M2
  combined fixtures, identical trace/manifest table projections, dense cell
  FlowIds/row pieces/header repetitions, old-profile table rejection, and
  zero-decoration publication matrix;
- validates MI3-07's exact four-profile descriptor, zero and complete-M2
  footnote fixtures, identical trace/manifest body/paint hashes, dense
  assignments and FootnoteFlowIds, split/multi-page carry closure, old-profile
  rejection, and fixed separator publication matrix;
- checks that `samples/invalid/expected-errors.json` indexes every invalid fixture;
- recomputes `config_sha256` from the effective TOML data model serialized with
  the supported RFC 8785 JSON Canonicalization Scheme subset;
- exercises every built/failed build-manifest conditional branch;
- validates the canonical machine capability snapshot, all generated machine
  expectations/matrices, and exact host-evidence shape;
- verifies config/trace/manifest compression, data-version, pass-limit, layout,
  selected-page-count, strict-fallback, and output-file relationships; and
- verifies file facts used by the minimal manifest and manifest-order fixture.

`package-config.schema.json` describes the fully merged 1.2 `EffectiveConfig` data
model that is hashed and passed to later phases. A user-authored `typaxis.toml`
is a partial input and is not validated directly against this schema. The
implementation first resolves defaults, the partial TOML file, environment
overrides, and CLI overrides; it then validates and serializes the resulting
complete `EffectiveConfig`. Its `allowed_uri_schemes` and `resource_roots`
arrays are unique and sorted by UTF-8 bytes. Raw 1.0 input receives the two
machine-input limit defaults before overrides and normalizes to the same 1.2
JCS bytes/hash as semantically equal raw 1.1/1.2 input. Canonical document packages also
write an explicit ordered-list `start`; an omitted source value resolves to
`1`, while unordered lists write `null`.

Each materialized trace state serializes the exact `pages`, `placed_anchors`,
`layout_epoch`, and `resolved_generated_text` that were used together for that
state. It never serializes a next-pass overlay in the current state. The next
pass chains from the previous materialized fingerprint, while its working
reference epoch comes from a sealed page/anchor/site-bound transition. The
portable schema validates the resulting state records; the in-process Rust
receipt is the trust boundary that prevents substituting an arbitrary store.
The initial state is likewise issued in process from the validated package and
limits; its wire record is not a constructor, and the reference workspace
accepts only the canonical empty overlay for a zero-site registry.

The effective limits include encoded per-font/per-image sizes, font/image
counts, and aggregate admitted `max_resource_bytes`; each per-resource encoded
byte maximum is no greater than that aggregate maximum.

Compact invalid fixtures may declare a `$fixture` base and a list of typed path
mutations. The validator materializes those mutations against the named minimal
artifact before Schema and semantic validation. `cross_artifacts` fixtures
materialize synchronized config, trace, and manifest copies and must remain
individually valid, so their indexed `rule_id` identifies exactly one broken
cross-artifact relationship.

Build manifests expose only terminal `built` and `failed` states. A built
output records the host-independent sink kind (`file` or `stdout`); the actual
file `HostPath` remains execution context and is never serialized. The sample
validator receives `samples/minimal/output.pdf` as an external fixture mapping
when it checks file-output byte and hash facts.

Portable DocumentPackage validation can check SourceId ownership and source
span bounds from each catalogued UTF-8 byte length, and it can fully check text
span/map bounds and UTF-8 boundaries because TextBuffer bytes are embedded.
Source code-point boundaries and identity-map byte-for-byte equality require
the admitted SourceCatalog bytes, which the canonical package intentionally
does not duplicate; those checks remain mandatory in the in-process
sealed parser owner before `ValidatedParsedPackage` serialization.

Before evaluating the recursive Document `$ref`, the validator iteratively
checks the configured `max_ast_nesting_depth` (profile maximum 64) across both
typed Document edges and valid StyleRule `extends` chains. This fail-closed
precheck prevents Python recursion failure and emits
`CROSS_LIMIT_AST_NESTING_DEPTH`; unknown or cyclic style parents keep their
dedicated conformance rule.

The invalid-fixture `rule_id` is a conformance-validator identifier, not the
public five-character Typaxis `DiagnosticCode` wire value. Schema rejection and
semantic conformance are separate validation layers; the bundled suite verifies
both expectations for every indexed fixture.
