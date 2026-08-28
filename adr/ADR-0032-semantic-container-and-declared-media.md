# ADR-0032: M4 semantic container and declared-media contract

## Status

Accepted on 2026-08-28 as the contract-versioning and semantic-container
decision gate for M4.

This ADR reserves a target contract and profile. It does not change the current
`typaxis.contract/1.3`, create a current Schema alias, register a public
descriptor, make `typaxis.contract/1.4` decodable by a public command, or claim
layout, PDF, CLI E2E, or release support. MI4-02 through MI4-12 may use the
reserved identities only through crate-private staging. MI4-13 is the sole
publication gate.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: this ADR fixes the base 1.4 shape and migration |
| implemented | No: the staging DTO, domain, flow, media attestation, and artifact owners are pending |
| public CLI E2E | No: public commands reject contract 1.4 and `production-book-1` |
| release-supported | No: publication and production evidence are MI4-13 |

## Context

The current contract can represent paragraphs, headings, lists, tables,
figures, footnotes, and advanced pagination, but it has no lossless node for a
result, proof, or exercise. Encoding one as a paragraph plus a class loses the
typed grouping boundary, child ownership, source mapping, and future structure
role. Removing the wrapper, concatenating its text, or rasterizing it would
also make selected-state and accessibility closure impossible.

Current image and font declarations name a logical resource and optional hash
but do not declare a media/container kind. PNG and TrueType support is derived
from admitted bytes. M4 adds vector, JPEG, and OTF/CFF families whose parser,
policy, manifest, and PDF plans must not be selected from a URI suffix or an
untrusted caller string. Making `media_type` required is a wire-shape change;
adding it to frozen contract 1.3 would violate ADR-0015.

The design inputs are docs/25 sections 7, 13.4, and 13.5; ADR-0015,
ADR-0027, ADR-0028, and ADR-0031; and invariants I-009, I-014, I-034, I-053,
I-063, I-065, I-067, I-068, I-073, and I-074. Existing source, text, style,
flow, selected-state, resource, Display, PDF, diagnostic, and terminal
publication rules remain normative unless this ADR explicitly narrows them.

## Reserved identities and publication boundary

The next unused minor identities are reserved as follows:

| Item | Identifier |
| --- | --- |
| target wire contract | `typaxis.contract/1.4` |
| target DocumentPackage Schema `$id` | `https://schemas.typaxis.invalid/1.4/document-package.schema.json` |
| target build-manifest Schema `$id` | `https://schemas.typaxis.invalid/1.4/build-manifest.schema.json` |
| target diagnostics Schema `$id` | `https://schemas.typaxis.invalid/1.4/diagnostics.schema.json` |
| target production profile | `typaxis.machine-pdf/production-book-1` |
| target profile receipt | `typaxis.production-book-profile-receipt/1` |
| semantic-container flow registry | `typaxis.semantic-container-flow-registry/1` |
| selected-container closure | `typaxis.semantic-container-selected-layout/1` |
| container structure binding | `typaxis.semantic-container-structure-binding/1` |

Contract 1.4 is an additive successor with new required members. The target
profile is immutable once MI4-13 publishes it, but its complete advertised
domain is assembled by the accepted M4 ADRs before that gate. A private 1.4
staging registry may grow during MI4-02 through MI4-12; it is not frozen or
public until it contains the complete adopted shape. No partial staging shape
may be exposed under the reserved ID.

The compatibility judgment applies to the complete planned M4 wire family:
the semantic container, ADR-0033 math/vector bindings, ADR-0034 document
metadata/language/outline facts, tagged-structure inputs, and required media
discriminators all belong to the new 1.4 boundary and none may be added to a
frozen 1.0 through 1.3 shape. Their assigned decision-gate ADRs may add exact
fields to the private 1.4 registry before MI4-13; this base reservation does
not pre-adopt later fields or their semantics. A planned field not adopted by
the publication gate is absent, while a wire addition after 1.4 is frozen
requires another contract migration.

At this ADR's adoption:

- current encoders, decoders, normalized config, diagnostics, manifests,
  traces, capabilities, help, fixtures, and top-level Schema aliases remain
  contract 1.3;
- the public accepted contract set remains exactly 1.0, 1.1, 1.2, and 1.3;
- the seven current profile descriptors and their canonical order are
  unchanged; and
- `typaxis.machine-pdf/paragraph-1` remains the default.

## Contract 1.4 semantic-container wire

Contract 1.4 adds one `block` alternative with this exact closed shape:

```json
{
  "anchor_id": null,
  "blocks": [
    {
      "children": [
        {
          "kind": "text",
          "node_id": 14,
          "span": {"end_byte": 6, "source_id": 0, "start_byte": 0},
          "text_span": {"end_byte": 6, "start_byte": 0, "text_id": 0}
        }
      ],
      "classes": [],
      "kind": "paragraph",
      "node_id": 13,
      "span": {"end_byte": 6, "source_id": 0, "start_byte": 0}
    }
  ],
  "classes": [],
  "kind": "semantic_container",
  "node_id": 12,
  "semantic_kind": "result",
  "span": {
    "end_byte": 6,
    "source_id": 0,
    "start_byte": 0
  }
}
```

The example uses canonical JCS member order. All seven shown members are
required and `blocks` has at least one item. ADR-0034 additionally permits the
sole optional `language` member on the assembled private target;
`additionalProperties` remains false over that combined property set.
`semantic_kind` is exactly one of:

```text
result
proof
exercise
```

`SemanticContainerKind` is a closed enum. There is no raw string escape,
namespaced extension value, `other`, custom role, or fallback kind in contract
1.4. A later kind requires a new contract and a new or explicitly migrated
profile. The container never generates a localized label such as “Proof” and
has no implicit title; an authored heading or paragraph child carries visible
label text and its own SourceSpan.

[ADR-0034](ADR-0034-document-metadata-language-and-outline.md) adds the
required nullable `anchor_id` during the still-private 1.4 assembly. A
non-null value makes the container an ordinary named-destination owner but
never an implicit outline entry; null means no container-owned destination.
It also owns the optional semantic `language` override and inheritance rules;
this base ADR assigns neither field an independent meaning.

The block is admitted anywhere the general DocumentPackage `block` type is
admitted: the document body, a list item, table cell, figure caption, footnote
definition, or another semantic container. It is not an inline and cannot
occur inside an inline `children` array. The separately restricted
header/footer page-region block grammar remains paragraph/heading-only, so a
container there is invalid. The production target accepts a container in the
document body or another semantic container and, when it advertises the
surrounding list/table/figure/footnote feature, in that feature's general block
slot. A profile may reject the surrounding feature as a whole, but it cannot
reinterpret the container as inline content or silently flatten a nested
container.

## Node, source, child, and style ownership

The semantic-container node owns its ordered `blocks` array. Its source span
mapping is explicit and never reconstructed from child text or layout. Global NodeId
allocation remains dense typed preorder: the container NodeId precedes every
descendant, and descendants retain the ordinary preorder of their block kind.
The container, its direct children, and every recursively contained descendant
must name the same SourceId. Every descendant SourceSpan is contained in the
container's half-open SourceSpan, and direct-child spans are nondecreasing in
wire order. Equal boundaries are permitted for structural wrappers, but a
reversed, out-of-owner, cross-source, gapped-NodeId, or foreign provenance edge
is not repaired.

The container is a styleable block. Contract 1.4 adds the closed selector block
type `semantic_container`; its normal unique UTF-8-byte-sorted classes apply
only to that node. There is no selector whose block type is a raw
`result`, `proof`, or `exercise` string. The computed owner type is
`SemanticContainerComputedStyle { semantic_kind, block_style }`, so layout,
Display, and PDF never compare the wire string. `typaxis-style` alone parses
the selector, applies cascade/inheritance/applicability, and issues that typed
computed style; the profile descriptor only selects the allowed property set.

The existing common block properties have these container meanings:

| Property | Container use |
| --- | --- |
| `font_family`, `font_size`, `line_height`, `text_align` | establish only their existing inherited values for descendant text; the wrapper paints no text |
| `space_before`, `space_after` | outer-flow glue before entry and after terminal exit |
| `start_indent`, `end_indent` | reduce the child-flow frame with checked positive remaining inline size |
| `keep_with_next` | binds the container's last painted fragment to the next outer painted block; it does not make the whole container unsplittable |
| `page` | retains the profile's existing typed page-selection semantics |

`width` and `keep_caption` are inapplicable. No border, background, label,
counter, title, or kind-specific raw style property is added by this ADR. The
three kinds have the same neutral visual initial values; the typed kind is
still retained in computed style, selected state, and structure binding.
Children perform their ordinary cascade using the container as the typed
inheritance parent. Container classes do not become child classes.

## Flow ownership and page splitting

Every validated semantic-container node owns exactly one independent FlowId,
bound to its NodeId, typed kind, parent FlowId and parent position, package,
profile receipt, style fingerprint, and LayoutEpoch. The parent flow consumes
one typed container-entry item and cannot resume after it until the container
flow reaches its exact terminal. The wrapper is therefore never discarded as
presentation-only grouping.

Child flow allocation follows one closed rule:

| Child | Flow rule |
| --- | --- |
| paragraph, heading, page break, or ordinary block Figure item | remains an item in the container FlowId; the container's typed enter/exit boundary is sufficient |
| list or table wrapper | remains one typed item in the container FlowId; its list-item or table-cell owners use the next row |
| list item, figure caption, table cell, or footnote definition | retains the already adopted independent subflow rule for that owner |
| nested semantic container | owns one nested semantic-container FlowId |
| inline descendant | remains in its owning paragraph/heading item and never receives a FlowId |

Thus ordinary child blocks do not receive gratuitous FlowIds, while every
owner that can carry an independent continuation has one. Allocation uses
canonical owner preorder, never worker completion, page, paint, or hash-map
order. Parent edges express ownership and must not merge nested continuations
into the container cursor.

The container may split only at legal break candidates produced by its child
flow. Each selected fragment records at least the container NodeId and kind,
FlowId, parent FlowId/position, dense fragment index, page/frame, exact
before/after cursor, first/last flags, computed-style fingerprint, and source
span. The first fragment opens the semantic boundary, the last closes it, and
intermediate page fragments continue the same boundary; pagination never
clones one container into multiple semantic nodes.

Every nonterminal selected fragment strictly advances the container cursor.
The parent advances past the container only after its terminal receipt. Empty
trailing frames are transparent, and the ordinary oversize policy applies
once when no legal positive fragment fits an empty full frame. Selected-state,
Display, manifest, and future structure-tree projections cover the same dense
fragment sequence. Missing, duplicate, reordered, wrong-parent, wrong-kind,
same-position, or post-terminal records are `I9190`.

## Outline and tagged-structure mapping

A semantic container does not create an outline entry and does not infer an
outline title from its kind, classes, or first child. Headings inside it remain
ordinary heading nodes. ADR-0034's explicit outline registry may reference a
heading or an anchored semantic container under its own NodeId and exact
AnchorId; it never derives an entry from kind, text, or selected coordinates.

For tagged structure, the typed mapping is reserved now:

| `semantic_kind` | Structure type | Required RoleMap target |
| --- | --- | --- |
| `result` | `/Result` | `/Div` |
| `proof` | `/Proof` | `/Div` |
| `exercise` | `/Exercise` | `/Div` |

One container produces one structure element even when it has multiple page
fragments. Its child structure nodes remain in canonical logical child order,
not paint-coordinate or page-object order. MI4-08 owns the complete structure
tree, marked-content, language, alternative, and validator policy, but it may
not change these semantic role names or flatten the wrapper. Private MI4-02
PDF observation may precede tagged-PDF publication only if the typed structure
binding survives to the later owner; it is not a public untagged fallback.

## Validation phases, diagnostics, and fallback

Validation is deterministic and fails at the earliest owner that has the
required authority:

| Condition | Phase and code |
| --- | --- |
| missing/extra member, unknown `semantic_kind`, inline occurrence, or structurally empty `blocks` | strict contract-1.4 decode, `P1102`, at the exact JSON Pointer |
| invalid NodeId, SourceId/SourceSpan containment, or canonical child ownership/order | sealed syntax validation, `P1102`, with package location and source note when available |
| nonempty array whose descendants are semantically empty, or parent/profile nesting outside the adopted domain | machine-profile preflight, `L5100`, before resource open or flow allocation |
| known but inapplicable style/property | style/profile preflight, `L5101` |
| profile or renderer without semantic-container support | capability preflight, `L5100`, before layout and PDF work |
| receipt, fragment, grouping, or structure contradiction | originating closure owner, `I9190` |

“Semantically empty” means that typed traversal finds no descendant capable of
producing authored text, a nonempty alternative-bearing replaced block, or a
nonempty owned subflow. Anchors, soft/hard breaks, page breaks, empty
paragraphs/headings, and recursively empty wrappers do not make a container
nonempty.

There is no path that converts a container to a paragraph, concatenates child
text, drops the wrapper, emits only its class, substitutes a PNG, or retries in
another renderer. An old profile rejects the typed node. A production profile
whose required structure/PDF implementation is unavailable is unavailable or
fails closed; it does not publish a reduced artifact under the same ID.

The container node and every descendant retain their existing
`max_ast_nodes` charge; owner depth, including each nested container edge, is
bounded by `max_ast_nesting_depth`; each independent FlowId is registered
before layout; and selected fragments use `max_fragments`. All maxima are
inclusive and max+1 is refused before allocation or work. This ADR adds no
synonymous configurable limit.

## Required media declarations in contract 1.4

Every contract-1.4 `resources.images[]` record adds required member
`media_type`; its base enum is exactly:

```text
png
```

Every contract-1.4 `resources.font_faces[]` record adds required member
`media_type`; its base enum is exactly:

```text
sfnt-truetype-glyf
ttc-truetype-glyf
```

These values deliberately match the existing closed capability vocabulary.
`sfnt-truetype-glyf` is a standalone sfnt with TrueType scaler and `glyf`
outlines; its `face_index` is zero. `ttc-truetype-glyf` is a TrueType
collection whose selected in-range face has TrueType scaler and `glyf`
outlines. The field is a typed format/container declaration, not a filename
extension or arbitrary MIME string.

[ADR-0033](ADR-0033-math-safe-vector-and-alternative-binding.md) adopts the
private `svg-safe-1` image value and assigns its implementation to MI4-04.
MI4-10 alone may add the adopted JPEG and OTF/CFF values. Neither decision may
rename or broaden the three base values. Any later media value after 1.4
publication requires a contract/profile migration.

Contract 1.3 and every earlier frozen ResourceCatalog shape remain unchanged
and forbid `media_type`. Contract 1.4 requires it even when the resource is
unused. Missing, null, wrong-typed, or unknown values are `P1102` during decode
and never become a default.

## Domain compatibility types and sole issuers

The trusted domain does not use a nullable or raw-string media field. It uses
these closed compatibility enums:

```text
ImageMediaDeclaration::{
  LegacyUnspecified,
  Declared(ImageMediaType),
}

FontMediaDeclaration::{
  LegacyUnspecified,
  Declared(FontMediaType),
}
```

`typaxis-document-package` owns the untrusted wire enums and versioned
decoder/encoder. `typaxis-document` owns the trusted `ImageMediaType`,
`FontMediaType`, declaration enums, and semantic-container node. Only sealed
`typaxis-syntax` lowering, after verifying frozen raw contract provenance, may
issue `LegacyUnspecified`. Contract-1.4 lowering can issue only `Declared`;
old-contract lowering cannot issue `Declared`; and a general caller cannot
construct a legacy value and attach new-contract provenance.

Encoding is equally closed. A legacy value may be omitted only by the exact
frozen encoder for its old contract. Encoding `LegacyUnspecified` as 1.4 is a
contract-migration error. Encoding a declared value into an old contract is
also an error; it is never dropped. A round trip therefore cannot turn absence
into declaration or declaration into absence.

`typaxis-machine-profile` owns the allowed declared-media set and issues a
policy receipt bound to profile, raw contract, complete declaration catalog,
package/session identity, and effective limits. Existing profiles retain only
their existing PNG and TrueType subset. The future production profile requires
raw 1.4 and rejects every `LegacyUnspecified` declaration with `R7100` before
opening resource bytes. It never asks resource admission to guess a value.

`typaxis-resource-admission` alone owns bytes-derived
`AdmittedImageMediaKind` and `AdmittedFontMediaKind`. After a stable read and
before expensive decoded-image allocation or font-outline evaluation, its
bounded decoder attests the actual container/outline kind and compares it to
the declared type under the policy receipt. The base exact mapping is:

| Declared type | Decoder-issued attestation |
| --- | --- |
| `png` | `AdmittedImageMediaKind::Png` |
| `sfnt-truetype-glyf` | `AdmittedFontMediaKind::SfntTrueTypeGlyf` |
| `ttc-truetype-glyf` | `AdmittedFontMediaKind::TtcTrueTypeGlyf` |

A known declaration disallowed by the profile is `R7100` before resource open.
A declaration/bytes mismatch is `R7100` after stable read but before the
media-specific expensive work above. URI suffix, host MIME metadata, caller
value, selected Figure, PDF subtype, or existing manifest record cannot issue
or override an attestation.

Candidate registration still happens before capability preflight so a failed
sidecar cannot overwrite a declared input path. Registration is not resource
open and does not create an attestation.

## Reference-source and `dump-ast` population

Reference TSF does not gain an authored media-type string. When the public
current exporter switches at MI4-13, source-mode `dump-ast --format json`
must perform the same stable resource admission and bounded decoder
attestation used by a build. It populates each 1.4 `media_type` only from the
same-session attestation mapped through the table above.

If a declared source resource cannot be admitted or attested, export fails
with the owning resource diagnostic before writing any JSON bytes. It does not
infer from `.png`, `.ttf`, `.ttc`, a configured family, or source syntax; it
does not emit `LegacyUnspecified`; and it does not fall back to contract 1.3.
The shared staging exporter may be tested privately before MI4-13 but cannot be
connected to public `dump-ast` early.

Machine-input round trips preserve the caller's typed 1.4 declaration and
later compare it to admitted bytes. They do not replace it with the attested
value. Creating a new 1.4 package from reference source and validating an
existing 1.4 package are distinct operations even when their typed values
match.

Because the CLI default remains `paragraph-1` and no old profile accepts raw
1.4, a public source-export round trip after MI4-13 must explicitly select
`--profile typaxis.machine-pdf/production-book-1` when the emitted package is
passed to `build-package` or `check-package`. Omitting the flag selects the
unchanged default and fails with `P1103` at `/contract`; it never upgrades the
profile implicitly.

## Manifest declaration and attestation

Only the target 1.4 M4 manifest resource branch adds
`media_declaration`. Each image or font resource record has one such tagged
union:

```json
{
  "kind": "declared",
  "media_type": "png"
}
```

or, only for the compatibility failure described below:

```json
{
  "kind": "legacy_unspecified"
}
```

`additionalProperties` is false. `kind = declared` requires the resource-kind
specific typed `media_type`; `kind = legacy_unspecified` forbids that member.
The separate `attested_media_kind` field contains the decoder-issued typed
kind and cannot be authored from the declaration. Its base wire values are
exactly `png` for images and `sfnt-truetype-glyf` or
`ttc-truetype-glyf` for fonts, so equality is a typed same-resource comparison
of those wire values. Contract 1.3 image records already require
`attested_media_kind = png`; contract 1.4 preserves that field name while
extending its typed image domain, and adds the corresponding field to the M4
font-resource branch. It does not rewrite the frozen image record or add a
font field to an old-profile Schema.

A built `production-book-1` manifest requires `kind = declared`, nonnull
`attested_media_kind`, and exact declaration/attestation equality for every
resource. A failed M4 manifest may use `attested_media_kind = null` only for a
sealed failure before resource admission, including an old-contract request
rejected by M4 preflight. Once resource admission begins, no partial resource
record is published until attestation succeeds; a read/decode failure remains
resource progress plus a diagnostic rather than a record with a null
attestation.

`legacy_unspecified` is legal in the new Schema only when an old raw contract
is requested with the M4 profile, sealed lowering has proved old provenance,
and pre-resource profile rejection is recorded. It is never legal in a built
M4 manifest, in a new-contract package, or as a synthesized result after an
attestation failure.

Frozen old-profile success and failure manifests do not gain
`media_declaration`, a font attestation field, or changed image-attestation
semantics. Existing image records retain their required
`attested_media_kind = png`; existing font records retain their old closed
shape. Existing raw-contract/profile goldens continue to use the 1.3 artifact
registry byte-for-byte. Raw 1.4 requested with an old profile is a new failed
1.4-envelope case: it is rejected with `P1103` at `/contract` before resource
open and records no M4 declaration or attestation facts. That failure is not
encoded as, and does not rewrite, a frozen old golden.

## Migration table

The default remains `typaxis.machine-pdf/paragraph-1`. “Old profile” below
means any of the seven profiles public before MI4. Their exact accepted raw
contract sets are frozen: none gains contract 1.4, even when a 1.4 package
contains only its previous semantic and media subset.

| Raw DocumentPackage contract | Requested profile | Before MI4-13 | After the MI4-13 gate |
| --- | --- | --- | --- |
| 1.0 / 1.1 / 1.2 / 1.3 | omitted or matching old profile | current frozen acceptance/rejection and 1.3 artifact bytes | unchanged; the frozen 1.3 artifact encoder/Schema preserves existing goldens |
| 1.4 | omitted or any old profile | public `P1103` because 1.4 is non-current | `P1103` at `/contract` before resource open; omission remains `paragraph-1`, and no old profile gains a new accepted contract |
| 1.4 | `production-book-1` | unknown profile usage and public `P1103` | accepted only for the complete M4 domain adopted before publication; every resource declaration is `Declared` |
| 1.0 / 1.1 / 1.2 / 1.3 | `production-book-1` | unknown profile usage | decoded with frozen provenance, then `L5100`/`R7100` before resource open; new failed manifest may record `legacy_unspecified` |
| 1.4 with missing/null/unknown `media_type` | any | public `P1103` | `P1102` during decode; it never becomes `LegacyUnspecified` |
| unknown | any | `P1103` or unknown-profile usage | same; no newest-contract/profile fallback |

After MI4-13, public `document_package_contracts` becomes exactly 1.0 through
1.4. The canonical profile suffix order becomes `basic-document-1`,
`columns-1`, `float-1`, `footnote-1`, `header-footer-1`, `paragraph-1`,
`production-book-1`, `table-1`. The default does not change, and each old
profile's accepted raw-contract set remains exactly frozen. A raw 1.4 package
must explicitly select
`production-book-1`; the current-export `dump-ast -> build-package` relation
therefore includes that explicit profile selection after publication.

Artifact version selection is explicit. An old raw contract with an old
profile continues through the frozen 1.3 artifact encoders. Any raw 1.4 input,
or any attempt to use `production-book-1`, uses the 1.4 artifact registry.
This is how old manifest bytes stay frozen while a pre-resource M4 failure can
still record provenance-bound legacy declarations under the new Schema.

The publication surfaces are therefore fixed independently:

| Surface | Before MI4-13 | After MI4-13 |
| --- | --- | --- |
| strict DocumentPackage decoder | public 1.0 through 1.3; raw 1.4 is `P1103` | public 1.0 through 1.4; old provenance lowers absence to legacy, while 1.4 requires declared media |
| DocumentPackage serializer and source `dump-ast` | public current 1.3; compatibility input is canonically checked under its raw version | version-exact compatibility encoders remain; the current serializer and source export switch to 1.4 and require same-session media attestation |
| default profile | `paragraph-1` | unchanged `paragraph-1`; raw 1.4 requires explicit `production-book-1` selection |
| old raw-contract/profile success or failure artifacts | current/frozen 1.3 registry | byte-identical frozen 1.3 encoder/Schema branch |
| raw 1.4 under an old profile | rejected as unknown contract | `P1103` at `/contract` in a failed 1.4 envelope before resource open; no M4 declaration facts |
| production-profile success or failure artifact | unavailable | `https://schemas.typaxis.invalid/1.4/build-manifest.schema.json`; success requires declaration/attestation equality, while pre-resource legacy failure uses the one allowed legacy/null branch |
| diagnostics | current `https://schemas.typaxis.invalid/1.3/diagnostics.schema.json` codes/locations | old artifact branch stays byte-frozen 1.3; the 1.4 artifact branch uses `https://schemas.typaxis.invalid/1.4/diagnostics.schema.json`, the same code meanings, and new typed package/resource subjects |
| capabilities and help | exact seven-profile 1.3 bytes | one atomic 1.4 update adds accepted contract/profile only after descriptor/fixture closure; default is unchanged |

## Atomic publication order

MI4-02 first creates an independent non-current 1.4 staging registry containing
the base declarations and container. ADR-0033 and ADR-0034, followed by later
M4 decision gates, extend that same private target only with their adopted
fields and enum values.
At every intermediate commit:

- public contract 1.4 input is `P1103`;
- `production-book-1` is absent from public parsing, help, capabilities, and
  normal dispatch;
- public `dump-ast`, current aliases, generated artifacts, and default remain
  1.3; and
- frozen 1.0 through 1.3 registries and old manifest fixtures are unchanged.

MI4-13 may publish only after validating the complete independent 1.4 registry
and all M4 receipt/PDF/evidence gates. One repository change set must switch or
register the contract enum and strict decoder, versioned encoder,
resource-attested `dump-ast`, normalized config, diagnostic and manifest
version dispatch, top-level Schema aliases, capability descriptor, normal
profile dispatch/help, combined fixture, and release matrices. It must remove
all private M4 runner/exporter entry points in the same change.

The decoder, a partial Schema registry, the profile ID, or one media enum may
not be published first. A failure halfway through implementation leaves all
public surfaces on 1.3. The complete `schemas/1.3/` registry is never populated
with a 1.4 definition.

## Closed rejection list

Contract 1.4 and the target profile reject at least:

- an open/custom/unknown semantic kind or a kind encoded only as a class;
- inline semantic containers, page-region containers, empty or recursively
  empty containers, cross-source descendants, and invalid owner nesting;
- a wrapper flattened into text, a paragraph, a class, or a raster image;
- implicit labels, outline entries, or structure roles inferred from visible
  text;
- grouping boundaries reconstructed from coordinates, pages, or PDF objects;
- missing, null, raw/custom, suffix-derived, or profile-disallowed media
  declarations;
- declaration/attestation mismatch or attestation derived from caller JSON;
- `LegacyUnspecified` under raw contract 1.4 or in an M4 success artifact;
- declared values silently omitted by an old encoder or legacy absence filled
  by a new encoder;
- new resource facts in frozen old-profile manifests; and
- any public contract/profile/Schema/CLI exposure before MI4-13.

## Rejected alternatives

1. Reuse contract 1.3. Required container and media fields change the frozen
   shape and would make old decoders/profile receipts ambiguous.
2. Encode result/proof/exercise as paragraph classes. Classes are presentation
   selectors, not typed ownership or accessibility roles, and cannot close
   child flows or page fragments.
3. Use an open namespaced kind. An unknown kind would have no immutable layout,
   outline, tagging, or fallback policy and would silently broaden a profile.
4. Make the container only a zero-cost grouping marker. Page splitting and
   nested subflows would then lose a continuation owner and could be flattened
   by downstream phases.
5. Infer media from URI suffix or accept a caller MIME string as attestation.
   Neither proves admitted bytes, selected font face, or outline technology.
6. Fill legacy declarations after resource decode. That would mutate old
   semantics and make old/new provenance indistinguishable.
7. Add media facts to old manifest Schemas. It would invalidate byte-frozen
   fixtures and make an old profile report a capability it never adopted.
8. Publish the 1.4 decoder before the complete profile. A decodable partial
   contract would become an observable promise with no lossless PDF path.

## Consequences

- MI4-02 has one implementation choice for the container record, flow owner,
  typed style, base media declarations, legacy compatibility enum, attestation
  mapping, and private exporter.
- Semantic grouping survives source mapping, pagination, Display, manifest,
  outline policy, and future tagged structure without forcing visible
  decoration or generated labels.
- The narrow closed kind enum means adding a semantic role is an explicit
  migration rather than an extension-string convention.
- Resource policy can reject known disallowed declarations before open and can
  diagnose declaration/bytes mismatch at the decoder boundary.
- Source export becomes more expensive because it must admit and attest every
  declared resource before writing contract-1.4 JSON; this is intentional.
- Old raw-contract/profile artifacts remain reproducible and byte-frozen, while
  new M4 failure evidence can distinguish legacy absence, declared type, and
  decoder attestation without null/string ambiguity.
- No user-visible feature changes at ADR adoption. The cost of atomic
  publication is deferred to MI4-13 rather than spread across staging tasks.
