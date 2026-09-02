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

## Adopted 1.4 migration target

[ADR-0032](../adr/ADR-0032-semantic-container-and-declared-media.md) reserves
the next minor identifier `typaxis.contract/1.4`, DocumentPackage Schema `$id`
`https://schemas.typaxis.invalid/1.4/document-package.schema.json`, and target
profile `typaxis.machine-pdf/production-book-1`. Contract 1.4 adds the closed
block-only `semantic_container` record and requires typed `media_type` on every
image/font-face declaration. Base values are `png`,
`sfnt-truetype-glyf`, and `ttc-truetype-glyf`.

[ADR-0033](../adr/ADR-0033-math-safe-vector-and-alternative-binding.md) extends
that same private target with explicit `inline_math`/`display_math`, required
`typaxis-math` version `1` source plus producer speech, and image media value
`svg-safe-1`. The additions have no old-contract encoding or early public
decoder; their exact receipt/vector semantics remain part of the MI4-13 atomic
publication set.

[ADR-0034](../adr/ADR-0034-document-metadata-language-and-outline.md) extends
the private target with a required closed metadata record, required document
language and explicit node-language overrides, an explicit source-bound
outline registry, and required nullable semantic-container anchors. It fixes
stable BCP 47 canonicalization/inheritance and deterministic PDF Info, XMP,
catalog language, and outline mapping. MI4-07 implements that complete private
slice; no current/frozen contract or public descriptor gains the fields early.

[ADR-0035](../adr/ADR-0035-tagged-pdf-structure-and-validation.md) closes the
same target's PDF/UA-1 structure projection, selected-paint/MCID/artifact
binding, `typaxis.book-xmp/2`, and exact independent-validation evidence. It
adds no DocumentPackage member or Schema bytes at adoption: existing 1.4
semantic, alternative, language, outline, grid, footnote, and link facts are
sufficient. MI4-09 implements the private manifest/expectation schemas and
tagged slice; current aliases, frozen registries, and public descriptors stay
unchanged.

[ADR-0036](../adr/ADR-0036-jpeg-and-opentype-cff-resource-profiles.md) adds the
exact image value `jpeg-baseline` and font value `sfnt-cff1` without changing
the existing PNG/SafeVector/TrueType values. The target resource descriptor
keeps five distinct immutable component IDs under
`typaxis.production-book-resource-set/1`. JPEG is the closed baseline-JFIF
Gray/YCbCr subset with deterministic APP0 removal and DCTDecode embedding;
`sfnt-cff1` is standalone `OTTO` plus name-keyed CFF1 with embedding-permission
preflight and a deterministic hint-stripped CID-keyed FontFile3/OpenType
subset. MI4-11 and MI4-12 implement these components separately in private
staging; no current/frozen Schema or public descriptor changes at adoption.

[ADR-0037](../adr/ADR-0037-producer-composed-math-vector.md) adds four explicit
producer-composed vector kinds and `svg-safe-2` to the still-unpublished 1.4
target. It preserves native math and `svg-safe-1` while versioning the
SafeVector component and complete production resource set to
`typaxis.resource-profile/safe-vector/2` and
`typaxis.production-book-resource-set/2`. The same target uses producer metric,
style, atomic inline/block flow/layout, content-key Form dedupe, math-vector
manifest `/1`, and book-navigation/tagged-PDF `/2` chains. MI4-V03 through
MI4-V18 may implement these only in private staging; MI4-V19 is feature-local
publication readiness and cannot switch aliases.

These are target facts, not current support. MI4-02 through MI4-12 and
MI4-V03 through MI4-V18 may extend only an independent crate-private 1.4
staging registry; MI4-V19 may add only evidence. Until MI4-13, public
contract 1.4 input remains `P1103`, the target profile is unknown to public
commands, top-level Schema aliases and canonical output remain 1.3, and the
seven-profile array/default remain unchanged. Frozen 1.0 through 1.3 registries
must not receive target definitions.

The domain compatibility representation is the closed
`LegacyUnspecified|Declared(typed media)` union. Only provenance-bound syntax
lowering may issue the legacy variant; raw 1.4 requires a declaration and the
production profile rejects legacy before resource open. Resource admission
alone issues bytes-derived media attestation and exact-matches it to the
declaration. Source-mode `dump-ast` may populate 1.4 only from that same stable
attestation, never a path suffix or source string.

MI4-13 is the only publication transaction and now depends on MI4-V19. It must
validate the complete independent 1.4 registry and atomically switch/register
decoder, encoder, resource-attested `dump-ast`, config, diagnostics, manifest
version dispatch,
current Schema aliases, capabilities, normal profile dispatch/help, fixtures,
and evidence. Old raw-contract/profile artifact encoders remain on frozen 1.3
so their manifest goldens do not acquire M4 resource fields; raw 1.4 or an M4
profile request uses the 1.4 artifact registry. The default remains
`paragraph-1`, every old profile's accepted raw-contract set remains frozen,
and raw 1.4 requires explicit `production-book-1` selection.

At that transaction the production resource set is `/2`, with exact image
media order `png, svg-safe-1, svg-safe-2, jpeg-baseline`; the `/1` resource set
and every SafeVector/navigation/tagged-PDF `/1` canonical record remain frozen.
Public capabilities remain the exact seven-profile 1.3 bytes until the same
transaction publishes the complete eighth descriptor. If 1.4 is published
before MI4-V19 completes, the producer-composed shape moves to a newly adopted
contract/profile rather than changing published 1.4.
