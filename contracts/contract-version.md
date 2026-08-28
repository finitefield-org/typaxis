# Typaxis contract version

Canonical current-output wire identifier:

```text
typaxis.contract/1.3
```

Current canonical JSON encoders and normalized `EffectiveConfig` values emit
this exact value. Top-level `schemas/*.schema.json` files are the current 1.3
aliases. The complete independent 1.3 registry is under `schemas/1.3/`;
the nineteen-schema 1.2 registry, eleven-schema 1.1 registry, and seven-schema
1.0 registry are frozen byte-for-byte in their version directories. A frozen
registry is never populated with a later shape. The design-package release is
`1.0.0`; the reference Rust workspace uses crate version `0.1.0`. Neither
version substitutes for the wire-contract identifier.

DocumentPackage input parsing recognizes exactly `typaxis.contract/1.0`
through `typaxis.contract/1.3`. The default
`typaxis.machine-pdf/paragraph-1` accepts all four identifiers while retaining
its frozen semantic subset. Explicit `basic-document-1`, `footnote-1`, and
`table-1` accept raw 1.2 and the exact neutral 1.3 encoding of the same frozen
semantics; raw 1.0/1.1 and non-neutral 1.3 are rejected. The public
`header-footer-1`, `columns-1`, and `float-1` profiles require raw 1.3.
Unknown identifiers never fall back to the current contract or newest profile.

Raw configuration input recognizes the same closed contract set. Earlier raw
configurations receive the defaults added by later compatible contracts.
Semantically equal 1.0, 1.1, 1.2, and 1.3 inputs normalize to the same current
1.3 `EffectiveConfig` before hashing. Compatibility input never changes the
producer rule: diagnostics, manifests, traces, display lists, normalized
configs, capabilities, and `dump-ast` output use 1.3.

Contract 1.2 adds the closed style-property names `space_before`,
`space_after`, `start_indent`, `end_indent`, `text_align`, `width`,
`keep_with_next`, and `keep_caption`, with the exact tagged values and semantics
adopted by [ADR-0028](../adr/ADR-0028-basic-document-profile.md). The publication
exposes the immutable `basic-document-1` profile, and MI3-04 subsequently
published the immutable `table-1` profile on the unchanged 1.2 table wire.
MI3-07 published ADR-0030's immutable `footnote-1` profile using the unchanged
1.2 definition/reference/page-master-footnote wire and conditional artifact
facts. `paragraph-1` remains the default and no older profile is broadened.

The contract remains a draft until the repository has a matching release tag
for the design-package release. An incompatible change to field meaning,
identifier space, coordinate unit, convergence semantics, extraction ownership,
path policy, or PDF resource ownership requires a new major value; a
backward-compatible additive wire change requires a new minor value. Editorial
changes that do not alter observable contract meaning do not increment the
value. A new shape must never be published under a frozen identifier.

## Published 1.3 migration

[ADR-0031](../adr/ADR-0031-advanced-pagination-profiles.md) adopted the minor
identifier `typaxis.contract/1.3` and DocumentPackage Schema `$id`
`https://schemas.typaxis.invalid/1.3/document-package.schema.json`. Contract
1.3 adds explicit horizontal/LTR page progression, trim, master-owned
header/footer content, column layout, and Figure `block`/`float` placement.
MI3-09 through MI3-11 implemented the three vertical slices privately; MI3-12
froze `schemas/1.3/` and switched the current aliases and encoders atomically.

The publication switched the contract enum/decoder, serializer, `dump-ast`,
normalized config, diagnostics, trace, manifest, capabilities, top-level
Schema aliases, profile dispatch, and public help in one repository change
set. Dedicated private advanced runners were removed; all three profiles use
the ordinary package pipeline and the default remains unchanged.

`paragraph-1` accepts raw 1.0 through 1.3 only for its frozen
semantic subset. `basic-document-1`, `table-1`, and `footnote-1` accept raw
1.2 and the exact neutral 1.3 encoding of the same frozen semantics; any
custom trim, page-region content, column layout, or floating Figure remains a
profile error. The new `header-footer-1`, `columns-1`, and `float-1` profiles
require raw 1.3. All canonical output uses 1.3, while 1.0, 1.1, and 1.2
registries remain independent and frozen. The default remains
`typaxis.machine-pdf/paragraph-1`; no raw contract or profile falls forward.
