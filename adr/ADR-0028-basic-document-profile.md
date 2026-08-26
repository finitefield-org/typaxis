# ADR-0028: Basic document machine-PDF profile

## Status

Accepted on 2026-08-27 as the target contract for M2.

This ADR is a decision gate. It does not make contract 1.2 current, register a
public profile, or claim implementation or release support. MI2-02 through
MI2-07 use crate-private staging entry points, and MI2-08 is the only milestone
that may publish the contract and profile.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: this ADR and `contracts/machine-pdf-capabilities.md` |
| implemented | No: M2 implementation slices are pending |
| public CLI E2E | No: public commands reject the new contract and profile |
| release-supported | No: the publication gate is MI2-08 |

## Context

`typaxis.machine-pdf/paragraph-1` is an immutable M1 profile. Lists, forced page
breaks, figures, links, and new block-style properties cannot be added to that
ID or to the frozen 1.1 DocumentPackage shape. M2 needs a second closed profile,
a new additive wire contract for typed style properties, and one common receipt,
limit, progress, and publication contract before its vertical slices begin.

The design inputs are docs/25 sections 8, 13.2, and 13.5. Existing trust,
determinism, diagnostic-budget, host-admission, and terminal-publication rules
from ADR-0027 remain normative unless this ADR explicitly narrows the new
profile.

## Identity and staging

The adopted identifiers are immutable:

| Item | Identifier |
| --- | --- |
| machine PDF profile | `typaxis.machine-pdf/basic-document-1` |
| DocumentPackage wire contract | `typaxis.contract/1.2` |
| versioned DocumentPackage Schema `$id` | `https://schemas.typaxis.invalid/1.2/document-package.schema.json` |
| flow-registry fingerprint algorithm | `typaxis.basic-flow-registry/1` |
| profile-receipt fingerprint algorithm | `typaxis.basic-profile-receipt/1` |

Before MI2-08, 1.2 Schemas live only in the versioned staging registry under
`schemas/1.2/`; `schemas/*.schema.json`, `typaxis_core::CONTRACT`, public decode,
public help, public capabilities, and all current generated artifacts remain
1.1. The private staging runner accepts the exact 1.2 contract and validates
staging trace/manifest artifacts against versioned, non-current 1.2 Schemas.
There is no hidden public option or alias for either new ID.

The 1.2 `dump-ast --format json` output contract is canonical DocumentPackage
JCS with `contract = "typaxis.contract/1.2"`, the existing coordinate unit and
dense ID rules, and the typed declaration values below. It never emits an
unknown property, an implicit default declaration, or a profile receipt. The
profile remains a build/check selection, not a DocumentPackage field.

## Closed accepted domain

The profile keeps the M1 source closure: exactly one admitted UTF-8 companion
source with `source_id = 0`, entry-only. It accepts only this domain:

| Axis | Accepted by `basic-document-1` |
| --- | --- |
| blocks in document body or list-item flow | `paragraph`, `heading`, `list`, `figure`, `page_break` |
| blocks in figure-caption flow | `paragraph`, `heading` only |
| paragraph/heading inlines | `text`, `anchor`, `reference(format = page)`, `soft_break`, `hard_break`, `link` |
| link children | the preceding inline set except `link`; at least one child must produce a painted cluster |
| list | ordered with explicit positive `start`, or unordered with null `start`; at least one item; nesting permitted within the configured limits |
| figure | one declared image, decoder-attested PNG, non-floating placement, required positive computed `width`, optional caption subflow and alt text |
| link targets | a package-local anchor or an admitted `http`, `https`, `mailto`, or `tel` URI |
| style selectors | `paragraph`, `heading`, `list`, `figure`, `page_break`, with the existing canonical class-selector grammar |
| style properties | `font_family`, `font_size`, `line_height`, `page`, plus the typed 1.2 properties below |
| page value/master | `page = auto`; exactly one default master, no selection rule, header, footer, or footnote frame |
| fonts | the `paragraph-1` TrueType sfnt/TTC `glyf` set |
| images | PNG only, proved from admitted bytes rather than URI suffix or a caller media string |
| PDF features | extractable text, named destinations, PNG image XObjects, internal GoTo links, and external URI annotations |

The following remain closed rejections: table, footnote definition/reference,
`emphasis`, `strong`, reference formats `text` and `number`, nested links, named
pages, multiple page masters, header/footer/footnote frames, JPEG/SVG/vector,
float/inline image, OTF/CFF, outline, tagged PDF, JavaScript/action annotations,
remote resource fetch, and every M3-or-later feature. A decoder being able to
represent a rejected kind is not profile acceptance.

Relative to frozen `paragraph-1`, the only accepted additions are block kinds
`list`, `figure`, and `page_break`; inline kind `link`; the eight 1.2 style
properties below; decoder-attested PNG declarations/use; and PNG XObject plus
internal/external link-annotation PDF semantics. Source closure, existing
paragraph/heading/inline/style meaning, page-master set, TrueType font set,
default profile, extraction, destination, trust, and publication semantics do
not change. This delta list is exhaustive.

## Contract 1.2 block-style extension

Contract 1.2 adds these exact declaration names to the closed property enum.
Lengths use signed 1/65536 PDF-point wire integers; the ranges below are
inclusive. `initial` is applied after cascade when no inherited or declared
winner exists. Inheritance follows the typed flow-owner chain, never geometric
containment reconstructed after layout. The existing cascade priority remains
`important`, selector specificity, matched rule source order, `extends` depth,
then declaration order.

Existing property applicability is also closed for the new profile.
`font_family`, `font_size`, and `line_height` apply to paragraph and heading
text and to a list's generated markers; a list must resolve the same complete
positive text style required by any other text-producing site. Item content
uses each child paragraph/heading's own computed style. Figure alt text is not
painted and takes no text style. `page` is accepted on all five block selectors
only with the existing `auto` value. Existing text properties on figure or
page-break, or any inline-level style property, are `L5101`.

| Property | Exact tagged wire value | Range / accepted keywords | Initial | Inherited | Sole layout consumer |
| --- | --- | --- | --- | --- | --- |
| `space_before` | `{"kind":"length","value":N}` | 0 through JSON-safe integer max | 0 | no | block-flow glue before a paragraph, heading, list, or figure |
| `space_after` | `{"kind":"length","value":N}` | 0 through JSON-safe integer max | 0 | no | block-flow glue after a paragraph, heading, list, or figure |
| `start_indent` | `{"kind":"length","value":N}` | 0 through JSON-safe integer max | 0 | no | flow frame start-edge inset before marker/content placement |
| `end_indent` | `{"kind":"length","value":N}` | 0 through JSON-safe integer max | 0 | no | flow frame end-edge inset before line/figure width resolution |
| `text_align` | `{"kind":"keyword","value":K}` | `start`, `end`, `center` | `start` | yes | paragraph/heading line placement; list marker labels are end-aligned independently |
| `width` | `{"kind":"keyword","value":"auto"}` or `{"kind":"length","value":N}` | `auto`, or 1 through JSON-safe integer max | `auto` | no | figure inline-size resolver; a figure with `auto` is rejected |
| `keep_with_next` | `{"kind":"boolean","value":B}` | `false`, `true` | `false` | no | paginator block/next-first-painted-fragment grouping |
| `keep_caption` | `{"kind":"boolean","value":B}` | `false`, `true` | `true` | no | figure image/caption-first-painted-fragment grouping |

`space_before` and `space_after` do not collapse: the checked sum of the
previous block's after-space and the next block's before-space is used.
`start_indent + end_indent` must leave a positive inline size. `text_align`
applies only to paragraph and heading line content. `width` and `keep_caption`
apply only to figure. `keep_with_next` applies to paragraph, heading, list, and
figure. Spacing/indent/keep declarations on `page_break`, or a known property on
an inapplicable selector, fail profile preflight with `L5101`; they are not
ignored.

Inter-block spacing is breakable glue. `space_before` is suppressed at the
start of every body frame, `space_after` is suppressed at document/subflow end,
and a page or forced boundary consumes pending glue without carrying it to the
next frame; spacing alone never creates a page. In horizontal layout,
`start_indent` and `end_indent` are logical insets (left/right for LTR and
right/left for RTL) applied before line or figure sizing. For `text_align`,
`start` places the line at logical start, `end` at logical end, and `center`
splits the nonnegative residual inline space by integer division, assigning an
odd final fixed-point unit to the logical end side. Negative residual or
checked arithmetic failure is `L5101` before fragment selection.

`keep_with_next = true` forbids a page boundary between the current block's
last positive-area fragment and the next painted block's first positive-area
fragment. The current block may split before its last fragment. Forced page
break is never skipped to satisfy keep; a keep immediately before one is
`L5101`. If there is no later painted block, keep is satisfied without creating
a blank page. If the protected pair cannot fit together in a complete empty
body frame, layout fails with `L5100` before selecting a state; the constraint
is not converted to a score or relaxed. The selected receipt records the two
NodeIds and their adjacent fragment IDs, so manifest/Display closure can prove
the boundary was not separated.

A raw 1.2 declaration name outside the complete old-plus-new enum, or a wrong
tagged value, is `P1102` at its declaration JSON Pointer. A known 1.2 property
that is unsupported by the requested profile or selector is `L5101` before
resource-byte admission or layout. Layout code consumes typed computed fields
and never compares property-name strings.

## List policy

- Ordered marker bytes are checked `u32` decimal `start + item_index` followed
  by `.`. Overflow is `L5100`. Unordered marker bytes are exactly U+2022.
- Each marker is one `GeneratedBufferKey`-owned buffer. Marker/content gap is
  exactly one computed `font_size`; it is layout glue, not marker text.
- A list marker is end-aligned in the widest marker column for that list. The
  list's `start_indent` precedes that column; `end_indent` reduces the item
  frame. Nested list indents are applied in their own child flow and are not
  accumulated by string or coordinate inference.
- Every list item owns a distinct child flow. An item without any positive-area
  painted content after typed traversal is rejected with `L5100`; its marker
  alone does not make the item nonempty.
- The marker and the item's first painted line are one keep group and fragment
  receipt. They move together to the next page. If that group cannot fit an
  empty body frame, the input is rejected with `L5100`; no marker-only or
  clipping fallback exists.

## Forced page-break policy

`page_break` is a typed boundary and has no Display paint operation. Pagination
starts with one open page. Consuming a break finalizes the current page, opens a
new blank page, and advances the flow cursor exactly once. Document finalization
keeps the open page. Therefore a leading break creates a leading blank page,
each additional consecutive break creates one intermediate blank page, a
trailing break creates a trailing blank page, and a document containing `N`
breaks and no painted content produces exactly `N + 1` blank pages. Returning
`More` with the pre-break cursor is an invariant failure (`I9190`).

## Figure, caption, and oversize policy

- The resource decoder, not the wire declaration or URI suffix, attests PNG and
  binds pixel width/height, canonical expanded decoded bytes, encoded hash, and
  `ImageResourceId` in the admitted ledger.
- Computed `width` is mandatory and positive. Height is
  `width * pixel_height / pixel_width`, rounded to nearest integer fixed-point
  unit with ties to even. Checked overflow or a rounded zero is `L5100`.
- The figure width must not exceed the indented body inline size and derived
  height must not exceed a complete empty body frame. Otherwise the figure is
  oversize and fails with `L5100` before pagination begins. The image is never
  clipped, scaled down, split, floated, or raster-substituted.
- Every figure owns a caption flow, including an empty terminal flow. With
  `keep_caption = true`, the image and first positive-area caption fragment must
  fit together; failure on an empty frame is `L5100`. With `false`, the image
  may end a page and the caption may start on the next. Remaining caption
  fragments may split normally. No keep relaxation is implicit.
- One selected figure produces exactly one `DrawImage`; usage, admitted ledger,
  finalization plan, XObject, and manifest image fact must agree on image ID,
  hash, dimensions, placement, and `attested_media_kind = "png"`.

## Link policy

- An internal target must resolve to one anchor in the same package and selected
  named-destination registry. Missing, duplicate, or foreign-package targets are
  `L5100` before Display/PDF construction.
- External URI schemes are the effective canonical subset of `http`, `https`,
  `mailto`, and `tel`. Syntax admission lowercases only the ASCII scheme; all
  bytes after the first colon are preserved. Empty values, disallowed or invalid
  schemes, control/whitespace/NUL, values over `max_uri_bytes`, invalid HTTP(S)
  authority, mail address, or telephone syntax are `L5100`. The PDF backend sees
  only `SafeUri`, never the raw string.
- Empty link children are rejected at preflight. After line selection, each link
  must own at least one positive-area painted cluster. For each selected
  page/line, all such cluster rectangles are unioned into exactly one page-local
  rectangle; zero-area unions are rejected. Records are ordered by link NodeId,
  page index, then line ordinal.
- Each rectangle becomes one annotation. Missing, extra, wrong-page, wrong-target,
  or out-of-page-bounds annotations fail closure before PDF publication. There
  is no plain-text or unlinked fallback.

## Existing-limit allocation and diagnostics

All maxima are inclusive. The owner consumes the budget before the max+1 work,
allocation, enqueue, read, or object-ID issue. No synonymous 1.2 limit field is
added.

| Subject | Existing limit and unit | Consume/check owner | Stable code |
| --- | --- | --- | --- |
| document/list/figure/link typed nodes and flow owners | `max_ast_nodes`, semantic nodes using the existing Document count (Document, blocks, list items, inlines, plus style declaration/value nodes) | syntax iterative admission before index or flow allocation; registry finish rechecks derived flow owners do not exceed admitted nodes | `P1120` |
| body/list-item/caption flow nesting | `max_ast_nesting_depth`, root body depth 1 and each child-flow owner edge +1 | syntax iterative depth precheck; flow-registry admission and finish recheck before recursive/stack layout | `P1121` |
| one generated marker | `max_text_buffer_bytes`, UTF-8 bytes in that marker buffer | generated-text owner before byte allocation | `T2100` |
| all parsed text plus one selected state's complete generated overlay | `max_text_bytes`, UTF-8 bytes | generated-text owner before marker insertion and selected-state seal | `T2101` |
| one PNG | `max_image_pixels`, checked `pixel_width * pixel_height` pixels | PNG admission before decode/allocation | `R7110` |
| one PNG canonical expanded buffer | `max_decoded_image_bytes`, decoded bytes | PNG admission before decode/allocation | `R7111` |
| selected link rectangles and all other materialized fragments | `max_fragments`, fragment/rectangle records in one layout state | pagination work-budget owner before fragment or rectangle issue | `L5110` |
| link annotations, image XObjects, and all other indirect objects | `max_pdf_objects`, indirect objects in one PDF | PDF backend complete role preflight before the first object ID/body allocation | `G6100` |

The first failure in canonical phase order is primary. Limit failures do not
retry with fewer features, fewer annotations, reduced image resolution, or a
different profile/backend.

The existing bounded convergence result (`converged`, `cycle_fallback`, or
`max_pass_fallback` with lowest-cost-then-earliest selection) remains in force,
but only among states that satisfy every hard marker, `keep_with_next`, and
`keep_caption` constraint. A state with a violated hard keep is inadmissible,
not a higher-cost candidate. Every forced break consumes once, and every
oversize/unsatisfiable keep reaches its terminal error once; neither may return
the same cursor as `More` or be reevaluated in another fallback state.

## Receipts, trace, and manifest closure

`BasicDocumentPreflightReceipt` extends the M1 capability binding with exact
contract ID, profile-receipt algorithm, resource-catalog fingerprint, and the
closed policy-table fingerprint. It remains package/admission-session-bound and
non-cloneable. `ValidatedFlowRegistryReceipt` binds the document body and every
list-item/caption flow in canonical Document preorder to owner NodeId, parent
FlowId, terminal, `LayoutEpoch`, package fingerprint, and
`typaxis.basic-flow-registry/1` hash.

The selected-state receipt binds the preflight receipt, complete flow registry,
all per-flow cursors/terminals, forced-break consume receipts, list marker keep
groups, figure placement/caption policy, link cluster rectangles, and the
resource ledger. Display, finalization, PDF, trace, and manifest accept only
this selected chain. Body-only binding is invalid.

Contract 1.2 staging trace and manifest use these exact additional facts:

- trace root `profile_receipt_sha256` and `flow_registry_sha256`;
- every flow-position record `flow_id`, `owner_node_id`, `parent_flow_id`, and
  `terminal` from the registry rather than caller allocation order;
- manifest `layout.profile_receipt_sha256` and
  `layout.flow_registry_sha256`, alongside the existing selected-state
  `final_fingerprint`;
- manifest image record `attested_media_kind = "png"` and exact admitted and
  selected-placement facts;
- manifest selected `list_flows`, `forced_page_breaks`, `figures`, and `links`
  arrays in canonical typed-owner order.

Registry count/depth, trace arrays, selected state, Display, and manifest must
cover every body and subflow exactly once. Missing, extra, wrong-owner,
wrong-parent, wrong-epoch, or wrong-terminal records are `I9190` and prevent
publication.

Machine progress keeps the ADR-0027 sequence and adds no caller-authored state:

```text
PackageValidated
  -> CapabilityValidated
  -> ResourcesAdmitted
  -> FlowRegistryValidated
  -> LayoutSelected
```

Unsupported profile content still fails after safe resource-candidate alias
registration but before resource bytes, layout, or PDF temporary output.
`check-package` ends after resources/style/geometry preflight and does not claim
pagination or PDF closure. Success publication order remains
`trace -> PDF -> diagnostics -> built manifest`; failure remains
`diagnostics -> failed manifest`; files are individually atomic, not a
multi-file transaction.

## Compatibility and atomic publication

The compatibility table after MI2-08 is:

| Raw DocumentPackage | Requested/resolved profile | Result |
| --- | --- | --- |
| 1.0 or 1.1 | `paragraph-1` | accepted as frozen compatibility input and projected to current 1.2 artifacts without changing profile semantics |
| 1.2 using only the frozen paragraph domain | `paragraph-1` | accepted; every 1.2-only property or M2 feature is still rejected by paragraph preflight |
| 1.0 or 1.1 | `basic-document-1` | `P1103` at `/contract`; no implicit upgrade of missing typed style values |
| 1.2 | `basic-document-1` | accepted only when the complete closed domain and policies above pass |
| any known contract | omitted `--profile` | resolves to `typaxis.machine-pdf/paragraph-1` |
| unknown contract/profile | any | contract `P1103` or CLI usage exit 2 respectively; never newest-version fallback |

Accepting a 1.2 encoding of the already-frozen paragraph semantic subset does
not broaden `paragraph-1`; its descriptor domain, policy, fixtures, and default
remain byte-for-byte/semantically frozen. `basic-document-1` is never selected
implicitly.

MI2-08 publishes atomically in one change set:

1. freeze the previous current 1.1 Schema registry under `schemas/1.1/` with
   drift hashes;
2. complete and validate the versioned 1.2 registry and positive/invalid
   fixtures without changing the frozen 1.0/1.1 registries;
3. switch the current contract constant, current Schema aliases, all generated
   config/package/trace/diagnostics/manifest/capability artifacts, canonical
   encoders, decoder registry, and `dump-ast` output to 1.2 together;
4. remove the crate-private staging entry point and make the normal pipeline the
   only implementation path;
5. advertise `basic-document-1` only after single-feature and combined fixture
   closure, two-run artifact equality, and documented-host gates pass.

No intermediate commit may expose a current 1.2 artifact with a 1.1 sibling,
accept the new profile through public CLI without its combined fixture, or alter
the contract 1.1 `default_profile`.

## Rejected alternatives

- Broadening `paragraph-1`: profile IDs are immutable closed contracts.
- Adding properties to 1.1: the property enum and tagged-value relation are a
  wire shape, so an additive minor contract is required.
- Publishing 1.2 before all M2 slices: partial capabilities would make one
  profile ID change meaning over time.
- Using image URI suffix as media proof, scaling/clipping oversized figures,
  dropping empty links, or rendering unsupported content as text: each would be
  a silent fallback outside the accepted domain.
- Adding flow, marker, rectangle, or figure-specific duplicate limit fields:
  the existing limits already have the required units and owners.

## Consequences

M2 implementers have one exact wire vocabulary, accepted domain, blank-page,
keep, oversize, URI, limit, receipt, artifact, and migration policy. The cost is
that some individually implemented features remain private until MI2-08; this
is intentional and prevents profile or contract meaning from changing in
place.
