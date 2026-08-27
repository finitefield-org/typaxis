# ADR-0031: Advanced pagination contract and profile split

## Status

Accepted on 2026-08-28 as the target contract for the M3 advanced-pagination
slice.

This ADR is a decision gate. It reserves contract and profile identities but
does not change the current contract, add a public Schema, register a public
descriptor, or claim implementation, CLI E2E, or release support. MI3-09,
MI3-10, and MI3-11 may expose the target only through crate-private staging
entry points. MI3-12 is the sole publication gate.

| Status axis | `header-footer-1` | `columns-1` | `float-1` |
| --- | --- | --- | --- |
| contract-defined | Yes: this ADR | Yes: this ADR | Yes: this ADR |
| implemented | No | No | No |
| public CLI E2E | No | No | No |
| release-supported | No | No | No |

At adoption, the public contract remains `typaxis.contract/1.2`; the public
profile array remains exactly `basic-document-1`, `footnote-1`, `paragraph-1`,
and `table-1`; and `paragraph-1` remains the default.

## Context

Contract 1.2 can describe a page size, body rectangle, optional auxiliary
rectangles, and page-master selection rules, but it cannot own header/footer
content, distinguish trim from media, declare column frames, or distinguish a
floating Figure from the immutable M2 block Figure. Adding any of those fields
under the frozen 1.2 identifier would violate ADR-0015 and the profile
immutability rule.

The design inputs are docs/25 sections 7, 8, and 13.1; docs/04, 08, 09, 10,
11, 14, 15, 18, and 25; ADR-0026; and invariants I-052, I-053, I-059, I-063,
I-066, and I-067. Existing package trust, fixed-point geometry, generated-text,
resource, convergence, selected-state, Display, PDF, and terminal-publication
rules remain normative unless this ADR explicitly narrows them.

## Immutable identities and profile split

The adopted identities are:

| Item | Identifier |
| --- | --- |
| target wire contract | `typaxis.contract/1.3` |
| target DocumentPackage Schema `$id` | `https://schemas.typaxis.invalid/1.3/document-package.schema.json` |
| target advanced-manifest Schema `$id` | `https://schemas.typaxis.invalid/1.3/machine-advanced-pagination-manifest.schema.json` |
| header/footer profile | `typaxis.machine-pdf/header-footer-1` |
| columns profile | `typaxis.machine-pdf/columns-1` |
| float profile | `typaxis.machine-pdf/float-1` |
| header/footer profile receipt | `typaxis.header-footer-profile-receipt/1` |
| columns profile receipt | `typaxis.columns-profile-receipt/1` |
| float profile receipt | `typaxis.float-profile-receipt/1` |
| advanced flow registry | `typaxis.advanced-flow-registry/1` |
| column-balance evaluation | `typaxis.column-balance-candidates/1` |
| float queue | `typaxis.float-queue/1` |
| selected layout | `typaxis.advanced-pagination-selected-layout/1` |
| paint closure | `typaxis.advanced-pagination-paint-closure/1` |
| trace/manifest projection | `typaxis.advanced-pagination-manifest/1` |

The three profiles inherit the complete `basic-document-1` content, style,
resource, and PDF domain, expressed on contract 1.3. They do not inherit table
or footnote behavior and do not compose with `table-1` or `footnote-1`.

| Profile | Additional accepted domain | Explicitly absent |
| --- | --- | --- |
| `header-footer-1` | custom trim; single or first/left/right master selection; static header/footer region content | columns and floats |
| `columns-1` | one or more sequential body columns; bounded balance on the final nonempty page | header/footer content and floats |
| `float-1` | nonwrapping direct-body Figure floats; one or more sequential columns without balance | header/footer content and column balance |

The split is intentional. One profile with three independently optional
feature families would require at least the eight presence/absence combinations
of header/footer, columns, and floats before considering first/left/right,
balance, and carry variants. The adopted split has three all-advertised
combined fixtures. `float-1` deliberately includes sequential columns because
float carry must be closed at a column boundary as well as a page boundary;
it rejects balance, so it does not inherit the column candidate-search state.
The split therefore tests the only required cross-feature edge without making
header/footer, balance, and float evolution mutually dependent.

Each profile also accepts its neutral form: no header/footer content for
`header-footer-1`, a null column layout for `columns-1`, and no floating Figure
for `float-1`. As with the published table and footnote profiles, fixture
coverage must contain both the zero-feature case and one complete
all-advertised case; accepting the neutral case does not allow another
profile's feature.

## Contract 1.3 DocumentPackage shape

Contract 1.3 is an additive successor to 1.2, but every new member below is
required in the 1.3 shape so omission cannot acquire an ambient default.
`additionalProperties` remains false at every object.

### Page-master additions

`page_masters` adds these required members:

```json
{
  "page_progression": "ltr",
  "writing_mode": "horizontal-tb"
}
```

Those are the only Schema values. They govern page/frame progression, not the
existing inline Unicode bidi algorithm. An RTL paragraph continues to use the
M2 inline bidi and logical-indent rules inside a physically left-to-right page
and column sequence.

Every `page_masters.masters[]` object adds these required members:

```json
{
  "column_layout": null,
  "footer_content": null,
  "header_content": null,
  "trim": {"height": 1, "width": 1, "x": 0, "y": 0}
}
```

The example values are illustrative, not defaults. `trim` is a positive
top-left Typaxis `Rect`. `header_content` and `footer_content` are each either
null or this exact page-region record:

```json
{
  "blocks": [],
  "node_id": 0,
  "span": {
    "end_byte": 0,
    "source_id": 0,
    "start_byte": 0
  }
}
```

The NodeId and SourceSpan values remain subject to the normal package rules.
The page-region root is a semantic node. Its `blocks` array admits zero or more
paragraphs or headings; their inlines are exactly `text`, `soft_break`, and
`hard_break`. A heading must have `anchor_id = null`. Classes and the unchanged
M2 paragraph/heading style properties remain available. Anchors, references,
links, footnote references, lists, figures, tables, page breaks, nested page
regions, generated page numbers, and running-expression fields are rejected.

At the portable 1.3 Schema layer, non-null `header_content` requires a non-null
`header`, and non-null `footer_content` requires a non-null `footer`. The
reverse is deliberately not a Schema rule: a non-null old rectangle with null
new content preserves a geometry-only 1.2 package during exhaustive 1.3
serialization without inventing a NodeId/SourceSpan or dropping geometry.
Every advanced profile is stricter and requires exact nullity between each
rectangle/content pair; a geometry-only pair is `L5101` at profile preflight.
An explicitly present content record with an empty `blocks` array is legal and
terminal-transparent; it is not normalized to null.

`column_layout` is null for one body column or has this exact shape:

```json
{
  "balance": "last_page",
  "count": 2,
  "fill": "sequential",
  "gap": 0
}
```

`count` is an integer in `2..=65,535`. `gap` is a nonnegative
`pdf_point_1_65536` length in the JSON-safe integer range. `fill` has only the
value `sequential`. `balance` has the Schema values `none` and `last_page`,
with the profile-specific restrictions below. Null, rather than an explicit
count of one, is the canonical one-column representation.

Contract 1.3 does not add an authored margin member. Physical margins are
derived from `trim` and `body`; a raw `margin`, `margin_top`, `bleed`, `crop`,
or similar member is `P1102`.

### Figure placement addition

Every contract 1.3 Figure adds required member `placement`, whose values are
exactly `block` and `float`. `block` retains the complete M2 Figure behavior.
`float` selects the closed float policy in this ADR. It does not accept a
placement object, side, wrap, clearance, priority, span, or fallback value.

When an old trusted package is encoded as 1.3, the shared encoder inserts
`placement = block` for every Figure, a full-media `trim`, null page-region
content, null `column_layout`, `writing_mode = horizontal-tb`, and
`page_progression = ltr`, while retaining every old header/footer/footnote
rectangle exactly. A geometry-only auxiliary rectangle remains portable but
is outside every 1.3 machine-PDF profile. No field is inferred from a path,
coordinates, profile name, or renderer result.

### NodeId and canonical document order

Contract 1.3 preserves every old NodeId when the additions are neutral. The
global dense NodeId preorder is:

1. the complete 1.2 Document tree and definition catalog in its existing
   canonical typed order;
2. masters in MasterId UTF-8 byte order;
3. the header region, then footer region, when present for each master; and
4. each region root followed by its block/inline typed preorder.

Column templates have no semantic NodeId. A floating Figure keeps its existing
Figure NodeId; placement never creates a clone. Duplicate, gapped, reordered,
or foreign-source page-region identities are `P1102` before flow allocation.

## Writing direction, page geometry, and PDF boxes

All page geometry uses checked `pdf_point_1_65536` integers and Typaxis
top-left coordinates. For selected master width `W` and height `H`:

- the PDF `/MediaBox` is `[0, 0, W, H]`;
- `/CropBox` is explicitly equal to `/MediaBox`;
- Typaxis trim `(x, y, w, h)` becomes PDF `/TrimBox`
  `[x, H-y-h, x+w, H-y]`; and
- `/BleedBox`, `/ArtBox`, `/Rotate`, and `/UserUnit` are absent.

The backend writes those three boxes in every selected page dictionary; it
does not infer them from Display bounds or inherit a different page-tree box.
All checked endpoints must be within the MediaBox and the trim must have
positive width and height.

The body must lie wholly inside trim. Its derived margins are exactly:

```text
top    = body.y - trim.y
right  = trim.x + trim.width - body.x - body.width
bottom = trim.y + trim.height - body.y - body.height
left   = body.x - trim.x
```

All four results must be nonnegative. A present header has the same `x` and
`width` as the body and lies wholly in the closed vertical interval from the
trim top to the body top. A present footer has the same `x` and `width` and
lies wholly between the body bottom and trim bottom. Header, body, and footer
may touch at an edge but may not have positive-area overlap. A rectangle,
endpoint sum, margin, or PDF-coordinate conversion that overflows or violates
these relations is `L5101`; it is never clipped, rounded, or reordered.

`header-footer-1` accepts any valid trim. `columns-1` and `float-1` require
trim to equal the MediaBox exactly; this prevents custom page-box policy from
silently composing with column or float behavior. All three profiles still
emit an explicit TrimBox.

## Header/footer selection, ownership, and repetition

`header-footer-1` requires `footnote = null` on every master and accepts exactly
two page-master-set forms.

The single form has one master, that master is `default_master_id`, and
`selection_rules` is empty. The first/left/right form has exactly three
distinct masters in MasterId UTF-8 byte order. The default is the right master
and the rule array is exactly:

| `source_order` | Role | `first` | `parity` | `named_page` |
| --- | --- | --- | --- | --- |
| 0 | first master | `true` | `any` | null |
| 1 | left master | null | `even` | null |

The remaining distinct master is the right master. Physical page number is
checked `page_index + 1`; the first master wins on page 1, later odd pages are
right pages selected by the default, and even pages are left pages selected by
the second rule. `page` remains `auto`, so `requested_named_page` is always
absent. No caller-selected MasterId, named page, odd rule, `first = false`,
section reset, alternate rule order, unused master, or fourth master is
accepted.

Each non-null page region owns one source FlowId bound to its region NodeId,
MasterId, kind, package, and LayoutEpoch. Selecting the master evaluates that
source flow from its start to its exact terminal in the selected rectangle.
The complete region must fit one occurrence; a nonterminal result in an empty
region frame is one terminal `L5100` oversize transition. Region content never
continues into another page and never changes the body cursor.

The same source flow is re-evaluated on every page selecting its master. It is
not cloned. A dense `repetition_index` starts at zero independently for each
`(MasterId, header|footer)` pair and increments only when that region is
selected. An empty present region still produces a terminal repetition/frame
receipt and no paint commands. A null region produces neither. The selected
page frame order is header when present, body, then footer when present; paint
uses the same order. An empty body still materializes exactly one page under
the existing blank-document policy, so its selected header/footer can paint.

## Sequential columns and bounded final balance

`columns-1` accepts one master, no selection rules, null header/footer/footnote
rectangles and content, and no floating Figure. Its `column_layout` is null or
uses `fill = sequential` and `balance = last_page`. `float-1` has the same
single-master restriction, but a non-null layout must use `balance = none`.

For body width `B`, count `N`, and gap `G`, the owner computes with checked
integers:

```text
total_gap = (N - 1) * G
available = B - total_gap
base      = floor(available / N)
residual  = available mod N
```

`available` and `base` must be positive. Columns `0..N-2` have width `base`;
the last has width `base + residual`. Starting at `body.x`, each next column
starts after the previous width and exactly one gap. Thus widths plus gaps
equal the body width exactly, and worker or map order cannot receive residual.
Column index increases physically left to right. There is no snake, reverse,
vertical, variable-width, span, or authored column-break policy.

Nonfinal pages always use full body height and fill column 0 through `N-1`
sequentially. A nonterminal column result must either strictly advance the body
FlowPosition or close a nonempty remainder and advance the frame ordinal. If
the next indivisible group makes no progress in an empty full-height column,
the owner emits one terminal `L5100` oversize result; it does not retry on an
identical column or next page. Once the body reaches terminal, every remaining
column frame records the same exact terminal before/after position with
`terminal = true` and no `More` result or paint.

### Final-page balance

For `columns-1`, the initial full-height sequential layout is uncharged. If it
is nonterminal, that page is selected unchanged. If it is terminal with no
positive content, no balance candidate is evaluated. If it is terminal with
positive content, all preceding pages remain fixed and the final page is
re-evaluated from its original entry cursor using equal candidate heights.

Let `H` be the full body height and `E` the checked sum of positive selected
block extents from the initial terminal layout, excluding unused trailing
space. The first target is `clamp(ceil(E / N), 1, H)`. A candidate lays out the
same input into `N` sequential equal-height frames. A terminal candidate is
selected. A nonterminal candidate must return a canonical rejection receipt
for every blocked column boundary, containing the next legal indivisible/keep
group and the strictly positive additional height needed to admit it. The next
target adds the lexicographically least `(deficit, column_index,
FlowPosition)` receipt's deficit. Candidate height must strictly increase and
must not exceed `H`.

The input fingerprint covers package/profile/LayoutEpoch, page/master/body
geometry, count/gap, final-page entry cursor, initial selected-fragment
fingerprint, and the complete rejection-receipt sequence. Repeating a
cursor/rejection fingerprint at a later candidate is balance oscillation and
emits `G6003` with no selected page. Returning a zero/nonincreasing deficit or
disagreeing with the initial full-height terminal result is an internal
`I9190` contradiction. There is no cycle selection or fallback.

Candidate evaluations consume
`ResourceLimits.max_column_balance_candidates`. The maximum is inclusive: the
candidate numbered exactly `max` may be selected. If it is still nonterminal,
the owner emits `G6003` before evaluating, allocating, or fragmenting candidate
`max + 1`; no page state is materialized. Discarded candidates cannot allocate
persistent IDs or consume selected `max_fragments` records. Only the selected
target and selected fragments reach Display, trace, manifest, or PDF.

## Float anchor, placement queue, and carry

`float-1` accepts `placement = float` only on a direct document-body Figure.
List-item, caption, page-region, table/cell, footnote, nested, or inline floats
are rejected. A block Figure retains M2 behavior. A floating Figure and its
complete caption form one unsplittable float; the caption retains the M2
caption block/inline/style domain and is not independently carried.

The float's anchor is the body FlowPosition immediately before its Figure
boundary, bound to the body FlowId and Figure NodeId. Reaching the anchor
atomically advances the body cursor past that Figure and appends exactly one
queue entry. A discarded page candidate commits neither action. Queue identity
and FIFO order are exactly `(body FlowId, anchor FlowPosition, Figure NodeId)`;
incoming entries precede newly encountered entries and a later entry never
bypasses the head.

Before enqueue, the layout owner proves that the Figure's computed width fits
the minimum selected column width and that its checked image-plus-caption
extent fits an empty full-height column. Failure is one terminal `L5100`
oversize result before queue allocation or body-cursor commit. No scale, clip,
rotation, caption split, alternate master, or block fallback is attempted.

The float creates a full-column-width exclusion band of its measured block
extent. The Figure itself is placed at the logical/physical left edge of that
band, with zero authored or implicit clearance. Body content resumes below the
band; no text occupies the same vertical band. There is no left/right side
wrap.

Only the queue head is evaluated. Its finite placement classes are considered
in this exact order whenever applicable:

1. `here`: once, at its anchor position, if the exclusion band fits the current
   column remainder;
2. `top`: at the current frame only when no body precedes it, or at the next
   unopened column/page, stacking FIFO entries downward;
3. `bottom`: in unused current-column space, stacking FIFO entries upward
   without displacing already selected body content; and
4. `next_page`: after the last eligible column on the current page, retain the
   entry and cross one physical page boundary.

An inapplicable candidate is recorded as rejected and does not cause reflow.
At a new column, the remaining head resumes with `top` before body layout. A
column transition does not increment page carry. At page start, incoming queue
entries are evaluated before newly discovered anchors. Successful placement
removes exactly the head and terminally consumes that Float FlowId; subsequent
heads are evaluated at the same scheduler point in FIFO order.

The initial anchor page has `carry_count = 0`. Sending an unplaced entry across
one page boundary increments it by one. A target page with
`carry_count == max_float_carry_pages` is still evaluated; failure emits
`G6004` before the crossing that would create `max + 1`. Queue size equal to
`max_float_queue` is accepted; appending entry `max + 1` emits `G6004` before
allocation, enqueue, or body-cursor commit. Entries are never dropped,
reordered, bypassed, or converted to blocks.

For each selected frame, paint order is top floats in FIFO order, body
fragments with `here` floats at their anchor order, then bottom floats in FIFO
order. Pages and columns remain ascending. Geometry is not used as a sorting
key. A page step must strictly advance the body cursor, terminally place at
least one float, or increment page carry for at least one queued float. An
unchanged composite `(body cursor, ordered queue/carry state, page, column)`
cannot issue `More` and is `I9190`. When body is terminal, pages continue only
while the bounded queue is nonempty; an empty terminal body and empty queue end
the document without a synthetic retry page.

## Canonical FlowId, frame, and selected order

The advanced registry extends, rather than flattens, the canonical flow model.
Its typed owner union and parent relations are:

| Flow owner | Parent | Source/progress rule |
| --- | --- | --- |
| document body | none | unchanged body cursor |
| existing list item/block-figure caption | unchanged containing flow | unchanged M2 rule |
| floating Figure | document body | separate unsplittable float terminal |
| floated Figure caption | that Float FlowId | advances only with the float placement |
| header/footer page region | document body | independent source start/terminal per repetition |
| column template | document body | frame-local view over the body source cursor |

The parent edge expresses ownership and depth, not cursor substitution. A
page-region, column, or float cursor is never serialized into the body
continuation. TableCell FlowIds and FootnoteFlowIds are absent from every
advanced registry: profile preflight rejects a table or footnote before flow
allocation, so no advanced flow may parent, or be parented by, either kind.

Dense FlowId allocation is independent of caller registration and worker
completion:

1. document body is FlowId 0;
2. document-descendant flows follow typed NodeId preorder; at a floated Figure,
   allocate its Float FlowId and then its caption FlowId;
3. page-region flows follow MasterId UTF-8 byte order, header before footer;
4. column-template flows follow MasterId byte order and ascending column index.

Null regions and null one-column layouts allocate no extra FlowId. A selected
null-layout body frame uses the document-body FlowId as both frame and source.
A non-null column frame uses its Column FlowId as `frame_flow_id` and the body
FlowId as `source_flow_id`; a page-region frame uses its region FlowId for both.
A column template has no NodeId and charges one effective AST unit before
allocation. Every owner, parent, source FlowId, terminal, package fingerprint,
profile receipt, and LayoutEpoch is included in
`typaxis.advanced-flow-registry/1`.

Selected pages are ordered by dense `page_index`. Within a page, frame order is
header, ascending body column, footer, omitting unavailable kinds. Header and
footer have null `column_index`; body frames have a dense zero-based index.
Within a frame, normal fragments retain logical source order and float
placements use the explicit paint ordinal above. Carry arrays retain FIFO
queue order. No hash map, coordinate sort, resource completion, or PDF object
order may break a tie.

## Limits, diagnostics, and terminal states

No new configurable limit is added. The exact ownership is:

| Work/record | Existing limit | Refusal |
| --- | --- | --- |
| masters and selection rules during decode | `max_style_rules` | `P1120` before max+1 record |
| page-region semantic nodes and column templates | `max_ast_nodes` | `P1120` before max+1 node/template |
| region/float parent depth | `max_ast_nesting_depth` | `P1121` before max+1 edge |
| materialized pages | `max_pages` | existing page-limit diagnostic before max+1 page |
| selected frames, region repetitions, float placements/carries, and existing content fragments | `max_fragments` | `L5110` before max+1 selected record |
| final-page balance evaluations | `max_column_balance_candidates` | `G6003` on oscillation or before max+1 candidate |
| simultaneously unplaced floats | `max_float_queue` | `G6004` before max+1 enqueue |
| page crossings by one float | `max_float_carry_pages` | `G6004` before max+1 crossing |

The normal page-master decode counter continues to share the existing
`max_style_rules` owner; there is no synonymous master, region, column, frame,
or repetition limit. Semantic nodes keep their ordinary charge and are not
double-counted merely because they paint repeatedly. Candidate-only fragments
are temporary work; selected records are charged atomically only when the
candidate commits.

Every maximum is inclusive under I-053. A refusal happens before reading,
allocating, enqueueing, evaluating, issuing an ID, or mutating selected state
for max+1. `P1102` covers unknown/wrong wire members and closed enum values;
`L5100` covers unsupported content/placement and terminal content oversize;
`L5101` covers unsupported page-master/style/geometry policy; and `I9190`
covers receipt, canonical-order, closure, or progress contradictions. MI3-12
adds public `G6003` and `G6004` diagnostic patterns and locations to the 1.3
registry in the same change set as their public profiles.

The terminal cases are therefore unique:

- an empty body produces the existing single page; unused column frames are
  terminal-transparent;
- an empty present header/footer repeats as a terminal frame with no paint;
- a region or indivisible body/float larger than its empty full frame produces
  one `L5100` and no retry;
- balance oscillation or an exhausted balance budget produces `G6003` and no
  selected final page;
- an exhausted queue/carry budget produces `G6004` and no crossing/enqueue;
  and
- a same-position `More` or inconsistent full-height result produces `I9190`.

## Receipt, trace, manifest, Display, and PDF closure

Each profile receipt is derived from its immutable descriptor and binds raw
contract 1.3, package/style/page-master/resource fingerprints, effective
limits, and the machine-input session. The flow-registry receipt binds all
owners and terminals. The selected-layout receipt binds the selected master,
PDF boxes, derived margins, every frame before/after cursor, repetitions,
column candidate result, float queue transition, and placed anchors. The paint
receipt binds the exact Display command ranges and PDF page/box observations
back to that selected layout. No JSON projection is an authority for creating
one of these receipts.

Contract 1.3 adds `machine-advanced-pagination-manifest.schema.json`. A built
advanced profile requires one top-level `advanced_pagination` member in both
layout trace and build manifest. The two members are byte-identical canonical
JCS projections with this closed root:

| Member | Meaning |
| --- | --- |
| `algorithm` | constant `typaxis.advanced-pagination-manifest/1` |
| `profile` | exact one of the three advanced profile IDs |
| `profile_receipt_sha256` | selected profile receipt |
| `flow_registry_sha256` | complete advanced registry |
| `selected_layout_sha256` | selected page/frame/queue closure |
| `paint_closure_sha256` | Display/PDF closure |
| `pages` | dense page records in ascending page index |

Every page record has required `page_index`, `master_id`, `media_box`,
`crop_box`, `trim_box`, `margins`, `frames`, `balance`,
`float_queue_before`, `float_placements`, `float_carries`, and
`float_queue_after`. PDF boxes are four-element
`[x_min,y_min,x_max,y_max]` arrays in PDF coordinates. Margins are the exact
`top/right/bottom/left` object above.

Each frame record has required `kind`, nullable `column_index`,
`frame_flow_id`, `source_flow_id`, top-left `rect`, `before_position`,
`after_position`, `terminal`, and nullable `repetition_index`. Each balance
record is null or has algorithm, input hash, dense candidate count, selected
target height, and balance-receipt hash. Each queue record has Float FlowId,
Figure NodeId, anchor body FlowId/position, and carry count. A placement adds
class, page, column, dense frame paint ordinal, bounds, and float/caption
terminal facts. A carry adds source page, target page, and incremented count.

The arrays use the canonical order fixed above; all IDs and indexes are dense
JSON-safe integers and all positions reuse the versioned FlowPosition shape.
`header-footer-1` has null balance and empty float arrays. `columns-1` has
empty float arrays and a non-null balance only on a selected nonempty balanced
final page. `float-1` has null balance. Missing, extra, reordered,
wrong-profile/master/page/frame/column/repetition/queue/carry/box/command facts
are `I9190` before PDF or manifest publication.

Old profiles forbid `advanced_pagination`, preserving their profile-specific
artifact records. A failed advanced build may contain the member only after an
owner-issued `LayoutSelected` receipt exists; earlier failure omits it. A built
advanced manifest always contains it. Trace and manifest never serialize
discarded balance candidates or an uncommitted float queue.

The 1.3 capability Schema adds an `advanced_pagination` object only to the
three new descriptor entries; old descriptor objects omit it byte-for-byte.
Its keys and exact values are:

| Profile | column count | balance | custom trim | header/footer | master selection | float classes |
| --- | --- | --- | --- | --- | --- | --- |
| `header-footer-1` | null | `forbidden` | true | true | `single`, `first_left_right` | empty |
| `columns-1` | 1..65,535 | `last_page` | false | false | `single` | empty |
| `float-1` | 1..65,535 | `none` | false | false | `single` | `here`, `top`, `bottom`, `next_page` |

The object also records `writing_mode = horizontal-tb`,
`page_progression = ltr`, and page boxes `crop`, `media`, `trim`. Count one is
encoded by null `column_layout`; non-null layouts start at two. Placement-class
order is semantic candidate order, not lexical order.

## Contract/profile migration and atomic publication

Before MI3-12, public decoders and commands reject raw contract 1.3 with
`P1103`, public profile parsing treats each new ID as unknown usage exit 2, and
current encoders, Schema aliases, capabilities, traces, manifests, diagnostics,
and `dump-ast` remain 1.2. Private staging may evolve under the reserved 1.3
target, but an incomplete staging registry is not current, is not copied to a
release, and cannot issue a public trusted 1.3 receipt. The complete 1.3
registry becomes frozen only at MI3-12.

MI3-12 applies this exact mapping:

| Raw DocumentPackage | Selected profile | After MI3-12 |
| --- | --- | --- |
| 1.0 / 1.1 / 1.2 | omitted or `paragraph-1` | accepted under the frozen paragraph semantic subset; output artifacts are 1.3 |
| neutral 1.3 paragraph subset | omitted or `paragraph-1` | accepted; trim is full media, auxiliary content/columns are null, and no Figure exists |
| non-neutral advanced 1.3 | omitted or `paragraph-1` | `L5100`/`L5101`; the default is never upgraded |
| 1.2 | `basic-document-1`, `table-1`, or `footnote-1` | unchanged accepted set for that exact immutable profile |
| neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | accepted as compatibility input with the same frozen profile semantics |
| non-neutral 1.3 | `basic-document-1`, `table-1`, or `footnote-1` | `L5100`/`L5101`; no advanced feature is ignored or downgraded |
| 1.0 / 1.1 | `basic-document-1`, `table-1`, or `footnote-1` | `P1103` at `/contract`; 1.2 semantics are not synthesized |
| 1.3 | matching `header-footer-1`, `columns-1`, or `float-1` | accepted only for this ADR's closed domain |
| 1.0 / 1.1 / 1.2 | any new advanced profile | `P1103` at `/contract`; no advanced members are synthesized |
| unknown | any | `P1103`; no newest-contract/profile fallback |

The default remains `typaxis.machine-pdf/paragraph-1`. For an old profile,
neutral 1.3 means full-media trim, null header/footer content, null
`column_layout`, and `placement = block` on every Figure, plus that profile's
unchanged master/auxiliary-frame rules. `footnote-1` retains its ADR-0030
footnote rectangle; the other old profiles retain their existing null
auxiliary rectangles. This is the same semantic-subset compatibility rule used
when `paragraph-1` accepted contract 1.2: it preserves `dump-ast ->
build-package`, changes no descriptor promise, and admits no advanced feature.

Existing profile descriptor objects and table/footnote projection algorithms
do not gain an advanced member. Their enclosing successful artifact uses
current contract 1.3 after publication, as every canonical output does. Their
accepted sets—1.2 plus only the exact neutral 1.3 encoding—and their
profile-specific descriptor/projection bytes are frozen by fixtures.

Publication is one atomic repository change set after every MI3-09 through
MI3-11 private gate passes:

1. validate the complete independent `schemas/1.3/` registry and revalidate
   the frozen 1.0, 1.1, and 1.2 registry hashes without cross-registration;
2. add 1.3 to the closed contract enum/decoder and switch current constants,
   top-level Schema aliases, normalized config, diagnostics, trace, manifest,
   capabilities, and all canonical encoders together;
3. make the serializer and `dump-ast` populate every required neutral/new
   member by typed exhaustive conversion, never by JSON patching;
4. register the three profile IDs, descriptors, preflight dispatch, public
   help, and normal pipeline only after descriptor/fixture bidirectional
   closure succeeds;
5. require `advanced_pagination` for built new profiles and forbid it for old
   profiles in both trace and manifest; and
6. remove all dedicated private runners/selectors and publish the three
   combined fixtures in `m3-all.json` only after external PDF,
   reproducibility, exact-limit, tamper, and documented-host gates pass.

After the switch, capability `document_package_contracts` is exactly 1.0,
1.1, 1.2, 1.3. Its profile array is UTF-8 byte ordered:
`basic-document-1`, `columns-1`, `float-1`, `footnote-1`,
`header-footer-1`, `paragraph-1`, `table-1`. There is no state in which a
public decoder accepts 1.3 while current Schema/encoder artifacts still claim
1.2, or a public profile can build without its manifest/PDF closure.

## Closed rejection list

The new profiles reject all of the following before selected layout:

- `vertical-rl`, `vertical-lr`, RTL page progression, right-to-left physical
  columns, and writing-mode-dependent page sides;
- named pages, arbitrary master rules, `first = false`, an odd-page rule,
  section page-number resets, running expressions, counters, or cloned region
  nodes;
- authored margins/crop/bleed/art boxes, region overlap, a region outside
  trim, or custom trim in `columns-1`/`float-1`;
- region anchors/references/links/lists/figures/tables/page breaks, region
  splitting/carry, and a header/footer combined with columns or floats;
- column width arrays, variable gaps, spans, forced column breaks, reverse or
  snake fill, balance on every page, unequal-height/optimal/unbounded balance,
  and balance combined with floats;
- nested/side/spanning floats, text wrap, nonzero/authored clearance, caller
  placement preference, head bypass, queue reorder/drop, partial float/caption
  placement, scaling, clipping, or block fallback; and
- tables or footnotes in any advanced profile, and advanced pagination in any
  existing public profile.

A future implementation needing one of these policies must adopt a new
contract or immutable profile as appropriate. It cannot reinterpret a null,
ignore an unknown member, or silently map the request to the closest adopted
policy.

## Rejected alternatives

Expanding contract 1.2 was rejected because header/footer ownership, trim,
columns, and Figure placement are observable additive wire semantics. A
unified `advanced-pagination-1` profile was rejected because its minimum eight
feature combinations and balance/float interaction would make one immutable
ID depend on several independently evolving algorithms. A profile for every
combination was rejected because it would encode optional-feature powersets
rather than stable implementation closures.

Treating header/footer as cloned body fragments was rejected because NodeId,
FlowId, progress, repetition, and generated content would be ambiguous.
Unbounded optimal column balance was rejected because it has no deterministic
work bound. Side-wrapping or priority floats were rejected because clearance,
shape exclusion, reorder, and starvation policies are absent from the wire.
Dropping or converting a deferred float was rejected because it changes
logical content.

## Consequences

MI3-09 can implement page boxes, canonical first/left/right selection, and
independent repeated region flows against a complete target. MI3-10 can
implement exact column partition and a bounded final-page candidate search.
MI3-11 can implement one FIFO nonwrapping float queue across both column and
page boundaries. None may change public current behavior before MI3-12.

The split costs three descriptors and combined fixtures, but each identifier
has one closed policy and one bounded progress proof. Producers can determine
from raw contract, selected profile, and capability descriptor exactly which
wire is accepted, while old profiles and all 1.2 Schema bytes remain frozen.
