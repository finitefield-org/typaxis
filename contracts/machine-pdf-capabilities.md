# Machine PDF capability contract

This document is the normative closed capability contract adopted by [ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md). It describes the M1 target; it is not evidence that the current CLI implements or advertises machine input.

## Status axes

| Axis | Current status |
| --- | --- |
| contract-defined | Yes: `typaxis.machine-pdf/paragraph-1` is fixed below |
| implemented | No: `typaxis-machine-profile` and its receipt are pending MI1-10 |
| public CLI E2E | No: `build-package`, `check-package`, and `capabilities` remain unregistered until MI1-17 |
| release-supported | No: only MI1-17 may change this status |

Portable DocumentPackage 1.0 Schema validation and current `dump-ast` export do not change these three negative delivery axes. A profile becomes available only when one implementation descriptor drives capability output, preflight, combined-fixture evidence, and the documented-host gate.

## Identity and default

- Profile ID: `typaxis.machine-pdf/paragraph-1`
- Contract 1.1 default profile: `typaxis.machine-pdf/paragraph-1`
- Source closure: exactly one source, `source_id = 0`, entry-only
- Unknown profile handling: usage exit 2; never fall back to the default or newest profile
- Manifest rule: record the resolved profile ID and require exact agreement with the preflight receipt

The profile ID is an immutable closed contract. Host availability and engine version do not alter its accepted domain.

## Closed accepted domain

| Axis | Accepted by `paragraph-1` |
| --- | --- |
| source | exactly one admitted UTF-8 companion source; entry-only closure |
| blocks | `paragraph`, `heading` |
| inlines | `text`, `anchor`, `reference(format = page)`, `soft_break`, `hard_break` |
| style properties | `font_family`, `font_size`, `line_height`, `page` |
| style selectors | `paragraph`, `heading`; another selector is rejected even if unused |
| page value | `auto` only |
| page master | exactly one default master; no selection rule; optional header/footer/footnote frames absent |
| fonts | standalone TrueType sfnt or TTC face with TrueType scaler and `glyf` outlines |
| font cardinality | zero is permitted only when no text-producing site requires a font |
| images | no declaration and no use |
| PDF features | extractable text and anchor named destinations |

A heading is a visual heading block. Its level and anchors remain in validation and fingerprints, but the profile does not promise PDF outline entries, tagged-PDF heading structure, heading-specific accessibility semantics, or a different fragmentation class from paragraph flow.

## Closed rejected domain

The same profile rejects, before resource bytes or layout are opened:

- list, table, figure, footnote definition/reference, and other block kinds;
- `emphasis`, `strong`, `link`, and non-page reference formats;
- named-page requests, additional page masters, selection rules, headers, footers, and footnote frames;
- image declarations or use, PNG/JPEG/SVG/vector content, math, and remote fetch;
- OTF/CFF fonts and fonts whose admitted bytes/metadata do not prove the declared TrueType `glyf` profile;
- link annotations, outlines, tagged PDF, and heading semantic structure;
- multiple companion sources or any inferred include closure;
- fallback to reference TSF, another backend, rasterization, or plain-text substitution.

An implementation accepting one of these items does not make it part of `paragraph-1`. It is a descriptor/implementation mismatch and must fail closed until a new profile is adopted.

## Descriptor and preflight ownership

`typaxis-machine-profile` owns one `MachineProfileDescriptor::PARAGRAPH_1`. The implementation must derive all of the following from that same descriptor rather than maintaining duplicate lists:

- canonical `capabilities --format json` profile fields;
- typed package preflight;
- positive and negative feature fixtures;
- the combined all-advertised-features fixture;
- producer-facing profile evidence.

Preflight consumes a sealed `ValidatedMachinePackage`, traverses typed Document nodes in NodeId order, then global style/page/resource items in canonical source-order/ID order. It materializes diagnostics within one command-wide budget but completes bounded traversal before deciding success. Unsupported content is an input failure, not an invitation to read resource bytes or call layout.

Success issues a non-cloneable `MachinePdfPreflightReceipt` bound to at least:

- resolved `MachinePdfProfileId`;
- `DocumentFingerprint`;
- `StyleFingerprint`;
- `MachineInputFingerprint`;
- opaque package/admission session identity.

Machine layout requires both `ValidatedMachinePackage` and the matching receipt. A swapped or forged receipt is an internal invariant failure; bare `ValidatedParsedPackage` plus a string profile ID is not a machine layout authority.

## Availability

Availability is compiled-host state, not profile meaning. `HostCapabilityDescriptor` combines tokens issued by the machine input owner, resource admission owner, and atomic publication owner.

Target capability JSON includes separate booleans for:

- `atomic_file_publish`;
- `contained_package_open`;
- `contained_resource_open`.

When any required token is unavailable, `profiles[].available` is `false`. Missing contained PACKAGE/resource open makes package commands fail with `I9110` / I/O exit 3 before PACKAGE bytes are read. Missing atomic publication instead fails publication-context construction with I/O exit 3 before a write receipt or target mutation; no diagnostics/manifest sidecar is promised when its publisher is unavailable. A security response may make an advertised profile unavailable for an engine version, but must not reuse the profile ID for a reduced or different accepted domain.

The capability artifact is generated from compiled descriptors only. It does not read config, filesystem contents, ambient locale, or per-job overrides. Built-in package byte/depth defaults and hard maxima come from the same core limit descriptor used by decode; effective per-job config remains bound separately by its config fingerprint.

The target 1.1 artifact publishes these descriptor facts from their sole constant/type owners:

| Fact | Value |
| --- | --- |
| coordinate unit | `pdf_point_1_65536` |
| accepted DocumentPackage contracts | `typaxis.contract/1.0`, `typaxis.contract/1.1` |
| `max_resource_roots` | 64 |
| `max_read_candidates` | 131,072 |
| `max_document_package_bytes` default / hard maximum | 134,217,728 / 9,007,199,254,740,991 |
| `max_json_nesting_depth` default / hard maximum | 256 / 256 |
| command-wide maximum diagnostics | 256 |

Exact maxima are accepted; max+1 is rejected before the associated allocation, open, read, work, or ID issuance. Host root/read-candidate and diagnostics caps are fixed security-profile constants, not per-job overrides.

## Compatible changes

The following changes are compatible with the same profile ID when they preserve observable semantics and existing fixtures:

- fixing an implementation bug so an already advertised item behaves as specified;
- improving diagnostic prose or adding non-normative notes without changing code meaning, location meaning, primary-error order, or side effects;
- performance improvements that preserve budgets, canonical ordering, bytes, and receipt checks;
- changing host availability from true to false for an engine/security condition while continuing to fail closed;
- adding evidence for an already advertised item without changing its promise.

## Incompatible changes

The following changes are incompatible and require a new profile ID or an explicit contract migration:

- adding a block, inline, reference format, style property/selector, page value/master behavior, font/image format, or PDF semantic feature;
- accepting any domain explicitly rejected above;
- changing single-source/entry-only closure or source ordering;
- changing default layout, pagination, fallback, blank-page, shaping, extraction, or publication policy;
- removing an advertised semantic feature while continuing to report the profile available;
- changing the default profile during contract 1.1;
- changing diagnostic code/location/primary-order meaning in a way that alters producer control flow;
- treating a host-availability difference as a different semantic interpretation of the same profile.

`paragraph-1` is never broadened in place. M2 and later capabilities require a decision-gate ADR that fixes a new profile ID, closed domain, limits, fallback/oversize behavior, publication semantics, fixtures, and migration rule before implementation begins.

## Contract and release gating

The target capability artifact uses `typaxis.contract/1.1`, but the repository remains on current generated contract 1.0 until MI1-14 performs the atomic migration. MI1-10 may implement an internal encoder; it does not expose a public command or current Schema. MI1-17 is the only milestone that may simultaneously register `capabilities`, publish the profile evidence, mark public CLI E2E complete, and claim release support.
