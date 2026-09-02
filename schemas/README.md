# Schema validation

## Delivery status and trust boundary

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| DocumentPackage portable shape | Yes, current 1.3 plus frozen 1.0/1.1/1.2 input | Yes: independent Schema/offline semantic validation | Yes, package commands | Yes |
| `dump-ast` DocumentPackage export | Yes, current 1.3 | Yes, shared converter/encoder | Yes, supported package round trip | Yes |
| sealed package/source admission | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, closed contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/basic-document-1` | Yes, ADR-0028 | Yes: full profile, receipt, multi-flow, style/list/break/figure/link closure | Yes, combined PDF/sidecars | Yes |
| `typaxis.machine-pdf/table-1` | Yes, ADR-0029 | Yes: grid/cell-flow, fragmentation/header, Display/PDF, trace/manifest closure | Yes, table-only and combined PDF/sidecars | Yes, MI3-04 gate |
| `typaxis.machine-pdf/footnote-1` | Yes, ADR-0030 | Yes: discovery/reflow, dedicated carry, Display/PDF, trace/manifest closure | Yes, zero and combined PDF/sidecars | Yes, MI3-07 gate |
| contract 1.1 Schema registry | Yes | Yes: frozen eleven-schema compatibility registry | Compatibility input only | Frozen |
| contract 1.2 Schema registry | Yes, ADR-0028/ADR-0029/ADR-0030 | Yes: frozen independent nineteen-schema compatibility registry | Compatibility input only | Frozen |
| contract 1.3 Schema registry | Yes, ADR-0031 | Yes: current aliases plus complete independent twenty-schema registry | Yes, seven public profiles | Yes, MI3-12 gate |
| contract 1.4 semantic-container/declared-media registry | Yes, ADR-0032 target | Yes: independent private staging registry and canonical slice fixtures | No; current aliases remain 1.3 | No, MI4-13 gate |
| contract 1.4 math/safe-vector extension | Yes, ADR-0033 target | Yes: MI4-04 SafeVector and MI4-05 math Schema/Rust/PDF slices in the private twenty-three-schema registry | No; current aliases remain 1.3 | No, MI4-13 gate |
| contract 1.4 metadata/language/outline extension | Yes, ADR-0034 target | Yes: MI4-07 private Schema/Rust/PDF/validator slice | No; current aliases remain 1.3 | No, MI4-13 gate |
| contract 1.4 tagged-PDF/accessibility extension | Yes, ADR-0035 target | Yes: MI4-09 private structure/marked-content/PDF/validator manifest slice | No; current aliases remain 1.3 | No, MI4-13 gate |
| contract 1.4 baseline-JPEG/CFF1 resource extension | Yes, ADR-0036 target | No: MI4-11/12 must add the separate private Schema/Rust/PDF/manifest slices | No; current aliases remain 1.3 | No, MI4-13 gate |
| contract 1.4 producer-composed math-vector extension | Yes, ADR-0037 target | Yes: MI4-V03 strict private Wire/Schema/domain and fail-closed legacy dispatch; validation/admission/layout/PDF remain staged | No; current aliases remain 1.3 | No; MI4-V19 then MI4-13 gate |

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
the public footnote profile, and [ADR-0031](../adr/ADR-0031-advanced-pagination-profiles.md)
defines the public advanced-pagination profiles.
[ADR-0032](../adr/ADR-0032-semantic-container-and-declared-media.md) reserves
the non-current 1.4 semantic-container/declared-media target and
`production-book-1`; MI4-02 implements them only in the private staging
registry and does not create a current alias or public descriptor.
[ADR-0033](../adr/ADR-0033-math-safe-vector-and-alternative-binding.md) fixes
the `typaxis-math` source/alternative receipt and `svg-safe-1` subset. MI4-04
implements the private SafeVector declaration/manifest/config slice and
MI4-05 implements the private math Schema/Rust/PDF slice.
[ADR-0034](../adr/ADR-0034-document-metadata-language-and-outline.md) fixes the
required closed metadata record, stable BCP 47 inheritance, explicit
source-bound outline, and Info/XMP/catalog/outline mapping. MI4-07 updated the
complete private 1.4 registry and all existing private fixtures atomically,
while current/frozen aliases stayed byte-identical.
[ADR-0035](../adr/ADR-0035-tagged-pdf-structure-and-validation.md) fixes the
PDF/UA-1 structure/marked-content/artifact projection, `book-xmp/2`, and exact
validator evidence. It adds no DocumentPackage member or Schema bytes at ADR
adoption; MI4-09 adds only the private versioned manifest/expectation
shapes needed for the implementation, without changing current/frozen aliases.
[ADR-0036](../adr/ADR-0036-jpeg-and-opentype-cff-resource-profiles.md) fixes
exact future `jpeg-baseline` and `sfnt-cff1` declarations, five distinct
resource component IDs, bounded decode/evaluation and embedding policy,
deterministic JPEG/CFF transforms, PDF plans, limits, and dependency identities.
ADR adoption changes no Schema: MI4-11 and MI4-12 own those private additions,
and MI4-13 alone may switch aliases or advertise the complete resource set.
[ADR-0037](../adr/ADR-0037-producer-composed-math-vector.md) adds the closed
private `inline_vector`, `math_vector`, `vector_figure`, and
`math_vector_block` shapes, `svg-safe-2` declaration/provenance, producer
metrics, and versioned SafeVector/resource-set, navigation, tagged-PDF, and
manifest branches. ADR adoption and MI4-V01 add no Schema shape. MI4-V03 adds
the strict private Wire/Schema/domain shape and keeps every legacy consumer
fail-closed; later V milestones own validation, admission, layout, and PDF only
inside the independent private 1.4 registry. MI4-V19 must
close feature evidence before MI4-13 switches aliases and advertises the exact
resource-set `/2`/vector capability projection.
`schemas/1.0/`, `schemas/1.1/`, and `schemas/1.2/` contain frozen
independent compatibility registries.
Top-level `schemas/*.schema.json` files are current 1.3 aliases, including
capability, fixture/matrix, and machine host-evidence Schemas. Current
generators emit 1.3, while the typed DocumentPackage parser recognizes 1.0
through 1.3. The public `build-package`, `check-package`, and capability CLI
surface supports all seven immutable profiles.

`schemas/1.3/` is the complete current versioned registry. It preserves the
1.2 artifact families described below and adds the required advanced package,
capability, trace, and manifest shapes. MI2-02 added
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
and forbidden for the other profiles. MI3-12 added the conditional
`advanced_pagination` projection, froze the complete 1.3 registry, and switched
the current aliases atomically.

The independent `schemas/1.4/` registry begins with MI4-02 private staging and
is not frozen until MI4-13. Its base DocumentPackage shape adds
the closed block-only `semantic_container` with
`semantic_kind = result|proof|exercise` and requires
the legacy private image branches `media_type = png|svg-safe-1` plus
`resources.font_faces[*].media_type = sfnt-truetype-glyf|ttc-truetype-glyf`.
MI4-V03 adds the separate `svg-safe-2` image branch with required nonnull hash
and provenance, and the four closed producer-vector kinds with their exact
metrics/source/alternative/number shape.
Current and frozen Schemas do not gain those fields. Missing/null/unknown 1.4
media values are decode failures rather than legacy absence or defaults.

ADR-0036 contract-defines later additions `jpeg-baseline` and `sfnt-cff1`,
but the private registry intentionally does not accept or advertise them at
MI4-10 adoption. MI4-11 must add only the JPEG enum/attestation/decoded/
sanitized/DCT-plan manifest facts. MI4-12 must separately add only the CFF
enum/attestation/table/license/glyph/subset/FontFile3 facts and the six private
font-limit members. Each addition must update every private 1.4 fixture and
semantic check atomically while leaving top-level/current and frozen 1.0-1.3
bytes unchanged.

ADR-0037 contract-defines `svg-safe-2`, four producer-vector kinds,
closed metrics/spacing/source/alternative records, and the version-2
SafeVector/resource-set/book-navigation/tagged-PDF artifact families. MI4-V01
contains only a TSV/SVG producer-interface corpus. MI4-V03 implements the
strict Wire/domain and applicable DocumentPackage Schema additions; V04 through
V17 own the remaining private Schema/artifact additions in
dependency order, MI4-V18 closes the crate-private combined fixture, and
MI4-V19 closes external evidence. Until MI4-13, no top-level alias, public capability Schema,
current encoder, or frozen registry contains those additions. Publication must
switch the complete resource-set `/2`, language/navigation `/2`, tagged-PDF
`/2`, manifest dispatch, profile descriptor, and Schema aliases in one change
set; a `/1` fallback is not a valid partial registry.

MI4-04 adds the private `machine-safe-vector-manifest` and a separate M4
effective-limit extension for vector nodes, path segments, nesting depth, and
math layout units. The SafeVector fixture closes every declared resource over
stable-byte hash, decoder attestation, canonical IR, selected use, Form plan,
and PDF object, including the adopted parser/IR/charge identities; an unused
admitted vector has no plan or PDF object. These
private limit members do not alter the current 1.3 `ResourceLimits` JCS or
fingerprint.

MI4-05 adds closed `inline_math`/`display_math` records and the private
`machine-math-manifest`. Its fixtures bind exact source/span, producer speech,
parsed AST, admitted MATH font, layout/vector paint, selected placement, PDF
`/ActualText`, and manifest observation while preserving 1.3 rejection and
public alias bytes. MI4-07 adds ADR-0034's metadata/language/outline fields and
book-navigation artifacts to this private registry. MI4-09 adds the private
`machine-accessibility-manifest`, dense logical structure/selected paint/MCID/
ParentTree closure, tagged PDF object observations, and the independent
writer-free validation projection. The current aliases remain byte-for-byte
1.3 and do not advertise or accept this target.

The private semantic-container manifest projection uses a closed
`media_declaration` tagged union. `kind = declared` requires typed
`media_type`; `kind = legacy_unspecified` forbids it. The separate
`attested_media_kind` is decoder-issued, nonnull, and equal on every built M4
record. Current image records already require that field with value `png`; the
1.4 target preserves the name and adds M4 font attestation without modifying
old resource shapes. Legacy/null is permitted only for a sealed old-contract
M4 request rejected before resource admission. Frozen old-profile
success/failure Schemas gain no declaration or changed attestation member and
retain their golden bytes. MI4-13 must validate the complete 1.4 registry
independently before switching any top-level alias.

Schema `$id` values under `https://schemas.typaxis.invalid/1.0/`,
`https://schemas.typaxis.invalid/1.1/`,
`https://schemas.typaxis.invalid/1.2/`,
`https://schemas.typaxis.invalid/1.3/`, and
`https://schemas.typaxis.invalid/1.4/` are logical, offline
identifiers. They are not fetch URLs. A validator must build independent
registries for each version and register every `*.schema.json` file by its
`$id` before resolving relative `$ref` values.

Run the bundled offline validator from the repository root:

```text
python3 schemas/validate.py
```

It requires Python 3.11 or later and `jsonschema` 4.18 or later. The validator:

- meta-validates the frozen 1.0/1.1/1.2, current/versioned 1.3, and private
  1.4 Draft
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
- validates MI3-07's frozen four-profile meanings, zero and complete-M2
  footnote fixtures, identical trace/manifest body/paint hashes, dense
  assignments and FootnoteFlowIds, split/multi-page carry closure, old-profile
  rejection, and fixed separator publication matrix;
- validates MI3-12's exact seven-profile descriptor, three advanced combined
  fixtures, bidirectional descriptor coverage, dense page/frame/queue progress,
  exact advanced limits, and the aggregate `m3-all.json` publication matrix;
- validates MI4-02's private 1.4 block-only result/proof/exercise nesting,
  required PNG/sfnt/TTC declarations, opaque-suffix resource hashes,
  declaration/attestation equality, dense selected fragments, and canonical
  Display/PDF/raster manifest projection while proving 1.3 rejection;
- validates MI4-04's private `svg-safe-1` declaration and SafeVector manifest,
  canonical fixture JCS, exact declared/stable-byte hash/attestation coverage,
  used-resource Form/PDF closure, unused-resource plan/object omission, unknown media
  rejection, attestation mismatch rejection, private M4 limit default/zero/
  hard-max-plus-one behavior, and 1.3 isolation;
- validates MI4-05's private math DocumentPackage/manifest shapes and canonical
  runner golden, including source/span/speech/font/vector/selected/PDF closure,
  exact and max+1 limits, typed tamper cases, and public 1.3 isolation;
- validates MI4-V03's private precomposed-vector DocumentPackage with all four
  kinds, exact `svg-safe-2` hash/provenance and source/resource closure,
  canonical JCS, conditional missing/null/forbidden/wrong-type negatives, and
  public 1.3 isolation;
- validates MI4-09's private accessibility DocumentPackage/manifest/PDF
  goldens, dense StructureNodeId/paint/page-local MCID/ParentTree closure,
  closed roles and validators, combined semantic coverage, typed tamper cases,
  and public 1.3 alias isolation;
- checks that `samples/invalid/expected-errors.json` indexes every invalid fixture;
- recomputes `config_sha256` from the effective TOML data model serialized with
  the supported RFC 8785 JSON Canonicalization Scheme subset;
- exercises every built/failed build-manifest conditional branch;
- validates the canonical machine capability snapshot, all generated machine
  expectations/matrices, and exact host-evidence shape;
- verifies config/trace/manifest compression, data-version, pass-limit, layout,
  selected-page-count, strict-fallback, and output-file relationships; and
- verifies file facts used by the minimal manifest and manifest-order fixture.

`package-config.schema.json` describes the fully merged 1.3 `EffectiveConfig` data
model that is hashed and passed to later phases. A user-authored `typaxis.toml`
is a partial input and is not validated directly against this schema. The
implementation first resolves defaults, the partial TOML file, environment
overrides, and CLI overrides; it then validates and serializes the resulting
complete `EffectiveConfig`. Its `allowed_uri_schemes` and `resource_roots`
arrays are unique and sorted by UTF-8 bytes. Earlier raw inputs receive later
compatible defaults before overrides and normalize to the same 1.3 JCS
bytes/hash as semantically equal raw 1.3 input. Canonical document packages also
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
