# ADR-0027: Machine DocumentPackage ingestion and immutable PDF profile

## Status

Accepted on 2026-08-25 as the target contract for M1.

This ADR fixes product and trust-boundary decisions; it does not claim that the target crates, `build-package`, `check-package`, `capabilities`, contract 1.1 artifacts, CLI E2E, or a release are implemented. The current public `build` input remains the bounded reference TSF until MI1-17 completes.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: this ADR and `contracts/machine-pdf-capabilities.md` |
| implemented | No: M1 implementation milestones are pending |
| public CLI E2E | No: machine commands remain unregistered until MI1-17 |
| release-supported | No: the release gate is MI1-17 |

## Context

The repository already has a portable DocumentPackage 1.0 Schema and `dump-ast --format json` export, but portable validation does not issue the sealed receipts required by `typaxis-syntax::ValidatedParsedPackage`. The current CLI accepts reference TSF, not DocumentPackage JSON, and cannot prove that package bytes, companion source bytes, resource bytes, profile preflight, and publication all belong to one host admission session.

M1 needs one decision covering command identity, host roots, source closure, trust ownership, capability semantics, contract migration, and side effects. Leaving any of these choices to individual implementation milestones would allow a schema-valid DTO or a caller-authored record to cross a trusted boundary.

The detailed design inputs are docs/25 sections 6, 10 through 12, and 13.5. This ADR adopts those M0/M1 decisions; later implementation milestones may choose private field/function names but may not change observable ordering, authority, or profile meaning.

## Decision

### Commands and input modes

The future public commands are distinct commands with distinct option types:

```text
typaxis build-package PACKAGE.json -o OUTPUT.pdf \
  [--package-root DIR] \
  [--profile typaxis.machine-pdf/paragraph-1] \
  [--config CONFIG] [--resource-root DIR ...] \
  [--strict] [--no-compress] [--max-<limit> N ...] \
  [--trace TRACE.json] [--trace-text] \
  [--emit-build-manifest MANIFEST.json] \
  [--emit-diagnostics DIAGNOSTICS.json] [--force]

typaxis check-package PACKAGE.json \
  [--package-root DIR] \
  [--profile typaxis.machine-pdf/paragraph-1] \
  [--config CONFIG] [--resource-root DIR ...] [--max-<limit> N ...] \
  [--emit-diagnostics DIAGNOSTICS.json]

typaxis capabilities --format json
```

- `build` and `check` remain reference-TSF commands. `build-package` and `check-package` consume DocumentPackage JSON. Extension or content sniffing never switches modes.
- `dump-ast --format json` remains a one-way export until the machine command E2E gate closes. It does not prove that the emitted JSON can be passed to the current `build` command.
- `check-package` runs host/package/source admission, strict decode, syntax lowering, capability preflight, resource metadata admission, and computed style/font-family resolution. It does not run pagination, complete glyph shaping, or PDF serialization.
- `check-package` does not accept output-only options such as `--strict`, `--no-compress`, `--trace`, `--force`, or a manifest target.
- Unknown profile IDs are usage errors. Unsupported content never falls back to reference TSF, another backend, raster output, or a newer profile.
- Omitting `--profile` resolves exactly to `typaxis.machine-pdf/paragraph-1`; reproducible producers should pass that ID explicitly.
- `build-package` shares current exact-`-` stdout, strict, compression, limit, target-alias, and individual atomic-publication semantics. `--trace-text` requires `--trace`.
- `capabilities` requires `--format json`; missing or unknown formats are usage exit 2. It reads no config, package, filesystem content, or ambient locale.
- After MI1-17, supported reference TSF -> `dump-ast --format json` -> `build-package` must preserve typed canonical JCS and `DocumentFingerprint`; raw JSON byte equality is not the round-trip criterion.
- The three machine commands remain absent from public help and dispatch until MI1-17 publishes them together with fixtures and producer documentation.

### Package root, sources, and resources

- If `--package-root` is omitted, the lexical parent of `PACKAGE.json` is the package root; an empty parent resolves to the current directory and the package URI is the file name. If the root is explicit, PACKAGE itself must be lexically root-contained before it is opened; an escape is usage exit 2.
- The root itself may resolve from a symlink to one canonical root handle. PACKAGE and source components beneath that handle must be opened no-follow and may not traverse a symlink. Config lookup/project-root semantics remain based on the current directory or `--config` parent and never switch implicitly to the package root.
- Canonical artifacts contain only the root-relative `PortablePath`; they never contain an absolute `HostPath` or canonicalized checkout path.
- M1 accepts exactly one companion source: `sources.length == 1` and `sources[0].source_id == 0`. Its declared URI, length, and SHA-256 must exactly match the stable admitted bytes.
- Every `SourceSpan` must be on a source-0 UTF-8 boundary, and identity `TextMap` ranges must match the admitted source bytes byte-for-byte. Replacement/inserted mappings keep the current typed mapping contract.
- Companion sources resolve only beneath the package root. A flat source array does not imply or reconstruct include edges.
- Font and image declarations resolve only against the explicit/configured resource-root set. The package root is not implicitly added as a resource root. A producer that wants the package directory to be the sole resource root must pass or configure it explicitly.
- Zero resource candidates is missing and two or more candidates is ambiguous even when their bytes are identical. Root order is inspection order, not first-match precedence.
- Multi-source input is rejected with a stable machine-source error until a new source-closure profile and ADR define its ordering and limits.

### Trust states, receipts, and ownership

Strict package acceptance is UTF-8 without BOM/raw NUL/trailing tokens; rejects escape-decoded duplicate keys and unknown fields at every depth; decodes integer fields without float coercion; checks JSON depth separately from typed AST depth; and preserves distinct raw-byte and typed-canonical JCS hashes. Package byte/depth, source/text, resource, and diagnostic budgets are independent and consumed before allocation or work.

`WireDocumentPackage` is a caller-constructible, untrusted DTO used for encoding and decoding shape. `DecodedDocumentPackage` is decoder-issued and carries a private binding to the exact admitted raw bytes and canonical JCS hash. A caller cannot attach a decode receipt to an existing DTO.

The trusted state progression is:

```text
Host PACKAGE / package root
  -> AdmittedPackageBytes
  -> SessionBoundDecodedPackage
  -> AdmittedMachineSourceSet
  -> AdmittedMachinePackage
  -> ValidatedMachinePackage { ValidatedParsedPackage, provenance }
  -> MachinePdfPreflightReceipt
  -> complete AdmittedResourceLedger
  -> layout / Display / PDF / terminal publication
```

Decoder-issued `DecodedDocumentPackage`, session-bound package/source receipts, `ValidatedMachinePackage`, `MachinePdfPreflightReceipt`, and publication receipts have private fields, no public raw-parts constructor, and no `Clone`. Exact byte/hash equality does not permit receipts from different sessions to be mixed. Failure paths return only the last sealed monotonic progress token; CLI or manifest code never reconstructs trusted facts from an untrusted DTO or error string.

Portable `MachineInputFingerprint` binds algorithm ID, raw/canonical PACKAGE identity, exact input contract ID, and companion source IDs/URIs/bytes/hashes. It excludes absolute host roots, opaque session identity, profile, and config. In-process receipts additionally compare the opaque session; later capability/output receipts bind profile and effective config separately.

The accepted dependency edges are:

```text
typaxis-host-admission      -> typaxis-core
typaxis-document-package    -> typaxis-core
typaxis-machine-input       -> typaxis-core + typaxis-host-admission
                               + typaxis-document-package
typaxis-syntax              -> typaxis-document-package + typaxis-machine-input
typaxis-machine-profile     -> typaxis-core + typaxis-syntax + typaxis-diagnostics
typaxis-resource-admission  -> typaxis-host-admission
typaxis-manifest            -> typaxis-host-admission + typaxis-machine-input
                               + typaxis-syntax + typaxis-machine-profile
typaxis-cli                 -> typaxis-document-package + typaxis-machine-input
                               + typaxis-syntax + typaxis-machine-profile
                               + existing resource/layout/display/pdf/manifest crates
```

`typaxis-machine-input -> typaxis-syntax` is forbidden. Syntax is the sole owner that can lower a decoded DTO and issue the existing private `ValidatedParsedPackage`. Generic component walking, same-handle snapshots, bounded stable reads, and read identities belong only to `typaxis-host-admission`; machine and resource crates bind its receipts to logical IDs and budgets instead of reimplementing host I/O.

### Initial machine PDF profile

The only M1 profile ID is `typaxis.machine-pdf/paragraph-1`. It is a closed immutable contract whose full accepted and rejected domain is defined by `contracts/machine-pdf-capabilities.md` and one implementation descriptor.

The profile accepts one entry-only source; paragraph and heading blocks; text, anchor, page reference, soft break, and hard break inlines; the closed initial text-style/page-master subset; and TrueType sfnt/TTC fonts with `glyf` outlines. It rejects list, table, figure, footnote, link annotation, image, SVG/vector, math, OTF/CFF, outline, and tagged-PDF promises.

A heading retains its level and anchors in validation and fingerprints and may be laid out visually as a paragraph-class flow. It does not promise PDF outline entries, tagged heading structure, or other heading semantics.

Adding a feature, accepting content previously rejected, changing the default layout/pagination policy, or changing source closure requires a new profile ID. The default profile remains `typaxis.machine-pdf/paragraph-1` for contract 1.1. A security-disabled implementation may report the profile unavailable, but it must not reuse the ID with reduced or different semantics.

### Host availability and capability output

Profile semantics and host availability are separate. A compiled `HostCapabilityDescriptor` supplies contained-package open, contained-resource open, and atomic-publish availability. The same tokens drive both `profiles[].available` in canonical capability JSON and command preflight.

If contained PACKAGE/resource open is unavailable, `build-package` and `check-package` fail with `I9110` and I/O exit 3 before PACKAGE bytes are read. If atomic publication itself is unavailable, publication-context construction fails with I/O exit 3 before a write receipt or target mutation; it cannot promise an `I9110` diagnostics/manifest sidecar. An unavailable host does not create a different meaning for `paragraph-1`. MI1-10 implements the internal descriptor/encoder, MI1-14 adds the current 1.1 capability Schema and artifact shape, and only MI1-17 exposes the public `capabilities` command and release evidence.

### Contract 1.1 migration

The repository's current generated contract remains `typaxis.contract/1.0` until MI1-14. MI1-14 is one atomic migration, not a series of public partial shapes:

- freeze the existing 1.0 Schema registry without changing its bytes or meaning;
- switch current generated config, trace, diagnostics, manifest, DocumentPackage export, and capability artifacts to 1.1 together;
- accept known DocumentPackage input contracts 1.0 and 1.1 with the same DocumentPackage shape, retaining the input contract ID in canonical package hashing;
- accept raw config 1.0/1.1 and normalize semantic values to generated 1.1 EffectiveConfig;
- add package identity, structured diagnostic location, capability, and machine limits with matching Rust types, Schemas, fixtures, validators, and docs;
- add manifest `input_profile`/`package_input` conditionals: reference mode uses `typaxis.reference-source/1` and always has `package_input = null`; machine mode uses the resolved machine profile; machine built has complete raw/canonical package facts; and machine failure projects only the last sealed progress;
- keep PACKAGE JSON out of companion `inputs`: that array contains admitted sources in SourceId order, while `package_input` alone owns PACKAGE identity;
- replace nullable diagnostic location fields with the 1.1 tagged package-JSON/source location union while keeping global I/O/publication location nullable;
- never emit a 1.1 artifact with an intermediate shape or silently relabel a 1.0 artifact.

Until MI1-14, 1.1 is target design only. Until MI1-17, even completed internal machine runners do not make machine input a public CLI or release-supported feature.

### Phase order and publication

Machine processing follows one deterministic order:

1. CLI/config/profile syntax and write-target validation.
2. Compiled host capability validation.
3. Package-root and PACKAGE contained open/stable read.
4. Bounded strict JSON lexical and typed decode.
5. Single companion-source admission.
6. Syntax lowering and sealed trusted-package issuance.
7. Registration of every safe resource candidate in the host read ledger, without opening resource bytes.
8. NodeId/global-order `paragraph-1` capability preflight.
9. Resource bytes/metadata admission and computed style/font-family preflight.
10. Layout, pagination, Display, resource finalization, and PDF graph construction.
11. Terminal publication.

Unsupported content is rejected before resource bytes, layout work, or PDF temporary output. Candidate registration still precedes capability preflight so diagnostics or failed manifests cannot overwrite declared input candidates.

Every output file is individually atomic; there is no multi-file transaction or rollback promise. Visible order is fixed:

- processing failure: diagnostics, then failed manifest; no PDF;
- file build success: trace, PDF, diagnostics, then built manifest;
- stdout build success: complete PDF stream, diagnostics, then built manifest;
- `check-package`: requested diagnostics only; no PDF, trace, or manifest.

Unrequested sidecars are never created. A publication error retains the exact already-visible artifact set and never describes an already-published file or stdout stream as rolled back.

All requested sidecar bytes and terminal preflight finish before the first visible publish. `--emit-diagnostics` success publishes `[]` or advisories; processing failure publishes an error/fatal set. If failure diagnostics publication fails, the terminal owner still attempts the already-preflighted failed manifest once and returns both outcomes. If success diagnostics publication fails after a visible PDF, it does not publish a built manifest and returns a typed partial-publication result.

## Alternatives rejected

- Overloading `build` with extension/content sniffing: it makes input identity and primary errors unstable.
- Treating Schema-valid or caller-constructed DTOs as trusted packages: it bypasses admitted source bytes and syntax ownership.
- Making the package root an implicit resource root: it silently broadens host authority and changes ambiguity behavior.
- Accepting multiple sources and inventing include edges from a flat array: the wire contract cannot prove that closure.
- Adding `typaxis-machine-input -> typaxis-syntax` or a public promotion API: either creates a dependency cycle or a second trusted-package issuer.
- Expanding `paragraph-1` in place: producers could no longer interpret one profile ID as an immutable promise.
- Reusing `typaxis.contract/1.0` for new manifest/diagnostic meaning or exposing partial 1.1 shapes: consumers could accept incompatible data under one ID.
- Falling back to reference TSF, raster, or another backend: it is lossy and hides unsupported content.
- Describing multiple atomic renames as one transaction: partial visibility and stdout writes are not rollbackable.

## Security consequences

- Host authority is explicit: package source access and resource-root access are distinct.
- Package, source, resource candidate, config, and write-target identities share one bounded read/write alias ledger before publication.
- Limits are consumed before allocation, open, read, or ID issuance; host root/read-candidate limits remain fixed compiled limits.
- Raw OS paths, error strings, and input snippets do not enter canonical diagnostics or manifests.
- Unsupported host capabilities and unsupported profile content fail closed before sensitive bytes or output temporary files are opened.
- Session-bound non-cloneable receipts prevent byte-equivalent substitution and caller-authored partial progress.

## Compatibility and migration

This ADR is editorial with respect to the current 1.0 CLI and artifacts: it adds no command and changes no current wire bytes. MI1-14 performs the incompatible/additive wire migration to 1.1 atomically. MI1-17 is the first milestone allowed to claim public CLI E2E or release support for machine input.

Diagnostic wording and notes may improve within one profile. Changes to diagnostic code meaning, location meaning, primary-error order, default profile, accepted domain, or layout policy require contract review and, where profile semantics change, a new profile ID.

## Rollout order

1. MI1-01 through MI1-09 establish crate boundaries, strict decode, host/source admission, syntax trust, and structured diagnostics without public commands.
2. MI1-10 creates the single descriptor, preflight receipt, host availability, and internal capability encoder.
3. MI1-11 through MI1-13 integrate layout preparation, sealed manifest progress, read/write aliasing, and terminal publication.
4. MI1-14 switches every current generated artifact to contract 1.1 atomically.
5. MI1-15 adds private command orchestration; MI1-16 closes internal fixtures and E2E.
6. MI1-17 registers all public commands, publishes producer guidance, runs documented-host gates, and is the first machine-input release-support claim.

M2 and later feature sets, profile IDs, page-break blank-page policy, table split policy, math/vector contracts, and book/release policies remain deferred to their dedicated decision-gate ADRs. Rejected alternatives in this ADR are not fallback implementation choices.

## Consequences

- Implementers have one phase order, owner graph, profile promise, migration point, and publication contract.
- Portable Schema validation, code implementation, public CLI E2E, and release support remain independently reportable status axes.
- M1 can reuse existing trusted layout/PDF types only after session-bound syntax and capability receipts close the machine-input boundary.
