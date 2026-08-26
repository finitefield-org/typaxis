# Schema validation

## Delivery status and trust boundary

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| DocumentPackage portable shape | Yes, current 1.1 plus frozen 1.0 input | Yes: dual Schema/offline semantic validation | Yes, package commands | Yes, M1 host gate |
| `dump-ast` DocumentPackage export | Yes, current 1.1 | Yes, shared converter/encoder | Yes, supported package round trip | Yes, M1 host gate |
| sealed package/source admission | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, closed contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| contract 1.1 Schema registry | Yes | Yes: eleven-schema current registry plus frozen seven-schema 1.0 registry | Yes | Yes, M1 host gate |

The offline validator proves portable Schema and semantic conformance only. It
does not issue in-process admission or validation receipts. The `typaxis build`
command intentionally does not accept `document-package.schema.json` instances;
the separate public `build-package` and `check-package` commands perform sealed
in-process admission. Their normative contract is [docs/26](../docs/26-machine-input-cli.md),
and remaining release evidence is tracked in
[docs/25](../docs/25-machine-input-pdf-improvements.md).

`WireDocumentPackage` is intentionally caller-constructible and untrusted.
Portable validation cannot manufacture the decoder-issued
`DecodedDocumentPackage`, bind raw and canonical package hashes to one host
session, admit the exact companion source bytes, or issue a
`ValidatedMachinePackage`/capability receipt. The internal trusted path calls the
strict decoder only on stable bytes owned by machine admission and lets sealed
`typaxis-syntax` perform source/TextMap/domain validation. A validator success,
`dump-ast` JSON, or matching hashes therefore never substitutes for an
in-process receipt.

[ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md) defines the
migration now implemented by the current registry. `schemas/1.0/` contains the
frozen seven-schema compatibility registry; top-level `schemas/*.schema.json`
contains the separate current eleven-schema 1.1 registry, including capability,
fixture/matrix, and machine host-evidence Schemas. Current generators emit 1.1
only, while the typed DocumentPackage parser recognizes 1.0 and 1.1. Public
`build-package`, `check-package`, and capability CLI E2E are available. Release
support is backed by the completed matching current-source Linux/macOS evidence aggregation.

Schema `$id` values under `https://schemas.typaxis.invalid/1.0/` and
`https://schemas.typaxis.invalid/1.1/` are logical, offline identifiers. They
are not fetch URLs. A validator must build independent registries for the two
versions and register every `*.schema.json` file by its `$id` before resolving
relative `$ref` values.

Run the bundled offline validator from the repository root:

```text
python3 schemas/validate.py
```

It requires Python 3.11 or later and `jsonschema` 4.18 or later. The validator:

- meta-validates both independent Draft 2020-12 registries and resolves every
  registered `$ref` without cross-registering versions;
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
- checks that `samples/invalid/expected-errors.json` indexes every invalid fixture;
- recomputes `config_sha256` from the effective TOML data model serialized with
  the supported RFC 8785 JSON Canonicalization Scheme subset;
- exercises every built/failed build-manifest conditional branch;
- validates the canonical machine capability snapshot, all generated machine
  expectations/matrices, and exact host-evidence shape;
- verifies config/trace/manifest compression, data-version, pass-limit, layout,
  selected-page-count, strict-fallback, and output-file relationships; and
- verifies file facts used by the minimal manifest and manifest-order fixture.

`package-config.schema.json` describes the fully merged 1.1 `EffectiveConfig` data
model that is hashed and passed to later phases. A user-authored `typaxis.toml`
is a partial input and is not validated directly against this schema. The
implementation first resolves defaults, the partial TOML file, environment
overrides, and CLI overrides; it then validates and serializes the resulting
complete `EffectiveConfig`. Its `allowed_uri_schemes` and `resource_roots`
arrays are unique and sorted by UTF-8 bytes. Raw 1.0 input receives the two
machine-input limit defaults before overrides and normalizes to the same 1.1
JCS bytes/hash as semantically equal raw 1.1 input. Canonical document packages also
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
