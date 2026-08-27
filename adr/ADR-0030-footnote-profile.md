# ADR-0030: Footnote machine-PDF profile

## Status

Accepted on 2026-08-27 as the target contract for the M3 footnote slice.

This ADR is a decision gate. It does not register a descriptor, make the
profile public, or claim layout, Display, PDF, CLI E2E, or release support.
MI3-06 may use only crate-private staging entry points. MI3-07 is the sole
publication gate.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: this ADR and `contracts/machine-pdf-capabilities.md` |
| implemented | No: discovery, reflow, carry, and paint slices are pending |
| public CLI E2E | No: public commands reject the profile ID |
| release-supported | No: the publication gate is MI3-07 |

Implementation update (2026-08-28): MI3-06 implemented the private
discovery/reflow slice and MI3-07 closed the publication gate. `footnote-1` is
now a public, implemented, release-supported profile. The table above and the
“before MI3-07” statements below intentionally preserve the decision-time
state against which the gate was evaluated.

## Context

The immutable `paragraph-1`, `basic-document-1`, and `table-1` profiles all
reject footnote definitions and references. Contract 1.2 already has the wire
shape needed for one Document-owned definition catalog, inline references, a
page-master footnote rectangle, generated footnote-marker sites, and the
existing footnote reflow limit. M3 therefore needs a closed interpretation of
that existing shape, not a silent broadening of a public profile or an
additive wire change.

The design inputs are docs/25 sections 8, 13.1, and 13.3; docs/04, 08, 09, 10,
and 18; and invariants I-012, I-014, I-041, I-053, I-059, and I-066. Existing
trust, deterministic-order, diagnostic-budget, selected-state, host-admission,
and terminal-publication rules remain normative unless this ADR explicitly
narrows the footnote profile.

## Identity and staging

The adopted identifiers are immutable:

| Item | Identifier |
| --- | --- |
| machine PDF profile | `typaxis.machine-pdf/footnote-1` |
| DocumentPackage wire contract | `typaxis.contract/1.2` |
| versioned DocumentPackage Schema `$id` | `https://schemas.typaxis.invalid/1.2/document-package.schema.json` |
| profile receipt | `typaxis.footnote-profile-receipt/1` |
| flow registry | `typaxis.footnote-flow-registry/1` |
| page evaluation fingerprint | `typaxis.footnote-page-evaluation/1` |
| selected layout | `typaxis.footnote-selected-layout/1` |
| paint closure | `typaxis.footnote-paint-closure/1` |

The current 1.2 DocumentPackage wire and Schema are sufficient. Before MI3-07,
the public `MachinePdfProfileId`, help, dispatch, capability JSON/profile
Schema, and normal package commands do not advertise or recognize
`footnote-1`. The public DocumentPackage Schema continues to recognize its
portable footnote wire without implying PDF support. MI3-06 staging must use
the exact public 1.2 decoder and typed AST, not a second DTO or a hidden public
selector. The public profile array remains exactly `basic-document-1`,
`paragraph-1`, `table-1`, and the default remains `paragraph-1`.

If MI3-06 or MI3-07 discovers that an authored marker, separator, split,
continuation, note-style, or frame-policy member is required, work stops before
that shape is implemented. A separate contract-migration task, new contract
ID, versioned Schema, compatibility table, and dependency edges must be added
first. Mutating contract 1.2 or treating a new member as implementation-only is
forbidden.

## Closed accepted wire and style domain

`footnote-1` preserves the complete `basic-document-1` content, style,
resource, and PDF behavior outside the footnote-specific reference, definition,
and master-region deltas below. It does not accept tables or change another M2
policy. A `footnote_reference` is additionally accepted in any paragraph or
heading in the document-body, list-item, or figure-caption flow where the
corresponding M2 inline set is accepted. Tables, table cells, and every
advanced-pagination domain remain rejected; `footnote-1` is not a composition
with `table-1`.

The definition subset is exact:

| Axis | Accepted by `footnote-1` |
| --- | --- |
| definition owner | one entry in `Document.footnotes`, in canonical FootnoteId UTF-8 byte order |
| definition blocks | one or more `paragraph` or `heading` blocks |
| definition inlines | the M2 paragraph inline set: `text`, `anchor`, `reference(format = page)`, `soft_break`, `hard_break`, and a non-nested painted `link` |
| nested note | no `footnote_reference` anywhere below a definition |
| definition style | unchanged paragraph/heading selectors and typed M2 properties; no footnote selector or property |
| body blocks/resources/PDF | exactly the `basic-document-1` domain outside notes |
| page master | one default master, no rules/header/footer, and the bounded footnote region below |

The definition must contain at least one text-producing inline reachable in
typed preorder: a nonempty `text` span, a page reference, or painted link
content containing one of those. An empty `blocks` array, only anchors/breaks,
or content that cannot produce a positive-area first line is an empty
definition and fails profile preflight with `L5100`; a marker alone does not
make it nonempty. A list, figure, table, page break, emphasis, strong, nested
link, non-page reference, or nested footnote reference below a definition is
also `L5100` before footnote flow registration or resource-byte admission.

No contract 1.2 declaration selects marker formatting, baseline shift,
separator geometry, note split, continuation text, inter-note gap, or footnote
frame behavior. A raw declaration or member attempting to author one is
`P1102`; a known current property on an inapplicable selector is `L5101`. No
alias is accepted and no unsupported value is rounded to this profile's fixed
policy.

## Definition closure and marker generation

The Document owns the definition catalog. FootnoteId remains an identifier, not
a display ordinal. Syntax validates the catalog in FootnoteId UTF-8 byte order,
requires each ID exactly once, and requires every reference target to resolve.
`footnote-1` additionally requires every definition to have at least one
reference in the complete typed body-flow tree. A duplicate definition or
missing target is `P1102`; an unreferenced definition is `L5100` at that
definition before flow allocation. Repeated references are accepted.

Marker ordinals are independent of page discovery. Definition ordinal is the
one-based position in the canonical FootnoteId-sorted definition catalog.
Both the definition marker and every reference to that definition materialize
the ordinal as shortest ASCII base-10 bytes, with no sign, leading zero,
punctuation, brackets, whitespace, or locale substitution. Thus catalog
`a, z` produces marker bytes `1, 2` even if `z` is referenced first. The marker
bytes are immutable initial generated text, not a pagination overlay update.

Each reference marker retains its reference NodeId as generated-site owner,
uses the nearest enclosing paragraph/heading as style owner, and is one atomic
ordinary-baseline inline at the exact reference position. This profile adds no
superscript, baseline shift, implicit space, or reference-marker keep beyond
the containing line.

Each definition has one distinct marker site owned by the definition NodeId.
Its canonical style owner is the first text-producing paragraph or heading in
typed definition preorder. The marker is painted only at the definition's
initial cursor, before its first source glyphs. It is kept with that first
positive-area line; the first content glyph origin follows the shaped marker
advance plus layout glue of exactly one computed `font_size`. Later lines and
continuation pages use the definition content's ordinary inline start and do
not repeat the marker. If marker, gap, and the first indivisible line cannot fit
the positive definition inline size, the definition is terminal `L5100`
oversize rather than clipped or restyled.

Repeated references paint their own marker sites but never clone, restart, or
repaint definition content. Exactly the earliest selected reference assigns
the definition; every later reference observes that assignment and contributes
no page reservation unless the same definition is already carried onto that
page.

## Master region, reservation, and separator

The one default master must have `header = null`, `footer = null`, and a
non-null footnote rectangle `M`. With checked coordinate arithmetic, `M` has
the same inline start and width as the body rectangle, the same block end, and
`0 < M.block_size < body.block_size`. It is the maximum bottom slice that may
be reserved from the body, not an independent overlapping layout surface.
Another origin, width, block end, empty region, full-body region, extra master,
or selection rule is `L5100` during profile preflight.

For a converged page with no definition fragment or incoming carry, reserved
height is exactly zero and no footnote frame or separator is emitted. Otherwise
the actual footnote frame has `M`'s inline bounds and block end and its checked
height is:

```text
reservation = separator_band + sum(selected definition fragment block extents)
```

All existing intra-definition block glue is already included in the selected
fragment extents. There is no additional inter-definition gap. The reservation
must be at most `M.block_size`; the body candidate uses the same body origin and
inline size with exactly `reservation` subtracted from its block end. Geometry
is never rounded, overlapped, clipped, or reconstructed from Display commands.

The fixed separator band is 65,536 `pdf_point_1_65536` units (1 pt). It contains
exactly one full-inline-width `StrokePath`: black `Gray(0)`, width 32,768 units
(0.5 pt), butt cap, miter join, miter limit 4.0, empty dash with phase zero. Its
centerline is 16,384 units after the actual footnote frame's block start, so the
stroke lies wholly inside the band. Definition content begins exactly at the
end of the band. There is one separator on every nonempty footnote page,
including a carry-only page, and none otherwise. Authored rules, colors,
lengths, gaps, or continuation separators require a new contract/profile.

## First-reference discovery and page assignment

Within one materialized global pagination state, logical reference order is
the canonical selected body-flow order: selected body fragments by FlowPosition,
then their typed inline preorder. It includes body-owned list-item and caption
subflows and never depends on registry insertion, worker completion, map order,
paint coordinates, or FootnoteId sorting.

For each page evaluation the owner performs these steps:

1. Start with incoming carries in their immutable global first-assignment
   ordinal. Each carry names one FootnoteFlowId and cursor.
2. Traverse the body candidate in logical order and record every reference
   occurrence with its reference NodeId. Deduplicate new IDs at their first
   occurrence on that page and discard IDs already assigned on an earlier page.
3. Append those new IDs after the carries in page-local first-reference order.
   The first newly assigned ID receives the next dense global assignment
   ordinal. The FootnoteId catalog order is not a tie-break.
4. Materialize definition subflows in that exact ordered set. A repeated
   reference on the same or later page never adds a second page assignment.
5. Derive the reservation and re-fragment the body until the fixed-point rule
   below succeeds.

New assignment ordinals are candidate-local until the page converges. Only the
convergence owner commits them to the materialized state's dense sequence; a
discarded evaluation cannot consume an ordinal or leave a gap. Incoming carry
ordinals were committed by the preceding converged page and are immutable.

Every reference occurrence remains in the discovery receipt even when its ID
is deduplicated. Each FootnoteId occurs at most once in one page's assignment
array. Across pages it occurs on its initial-assignment page and on every page
carrying unfinished content, but not merely because a later repeated reference
was painted. The union across the selected state is exactly the referenced
definition set, which is also the complete catalog because unreferenced
definitions are rejected.

The compatibility `PaginationFingerprintRecord.page.footnote_ids` projection
remains FootnoteId UTF-8 byte sorted as required by docs/09. The ordered
footnote receipt separately binds assignment ordinal, reference owner, paint
order, and carry; neither representation may be reconstructed from the other.

## Definition splitting, carry, and keep interaction

The fixed split policy is `allow`. Paragraph lines, marker-plus-first-line,
and existing hard `keep_with_next` groups are indivisible; every other existing
paragraph/block boundary is legal. There is no authored `forbid` or
`force_if_oversized` selector. Body keep groups are evaluated against the
reduced body frame and definition keep groups against the actual footnote
frame. Neither is converted to a score, weakened, or dropped to obtain
convergence.

For one evaluated ordered set, let each active definition's next indivisible
fragment be its minimum progress. The owner first reserves the separator and
one minimum-progress fragment for every incoming carry and new assignment. If
the total exceeds `M.block_size`, trailing new assignments are removed only by
choosing the greatest legal body break before their first reference and
reevaluating the body. Incoming carries are never dropped. If a body line has
multiple new references whose combined minima cannot coexist, or the incoming
carry minima alone cannot fit an otherwise empty maximum footnote region,
layout terminates with `L5100`.

After all minima fit, remaining capacity is exactly `M.block_size` minus the
separator and those minima and is distributed in ordered-set order. For each
definition, choose the greatest legal prefix that fits while retaining the
already measured minimum for every later active definition. This is the sole
split tie-break. It may leave multiple unfinished definitions, but every
selected definition cursor strictly advances. Empty progress, a forced cut,
scale, clip, overlap, reordered priority, or retry of the same cursor is
forbidden.

An unfinished definition issues one dedicated carry containing profile/package
identity, FootnoteFlowId, global assignment ordinal, source page, next page,
and exact before/after/next cursor. The next page processes carries before new
references in assignment order. Carry state never enters the body cursor and
does not repeat a definition marker or synthesize continuation text. A page may
be carry-only when at least one footnote cursor advances. Missing, duplicate,
resurrected, reordered, wrong-page, wrong-flow, or nonadvancing carry is
`I9190`.

Composite page progress is either a strict body-flow cursor advance or at
least one strict FootnoteFlow cursor advance with every other active cursor
unchanged or advanced. A carry-only transition retains the held body cursor in
the page-state receipt; it does not ask the body Fragmenter to issue
`Continuation::More` at the same position. Thus per-flow invariant I-014
remains intact while body and footnote progress stay independently typed.

The first positive definition fragment must occur on the page of its assigning
reference. If reservation cannot preserve both the reference's indivisible
body line/keep group and every required minimum in an otherwise empty body plus
maximum footnote region, the definition/reference pair is terminal `L5100`
oversize. Later repeated references do not create this keep. `max_pages` and
strict cursor progress bound long continuations; `max_float_carry_pages` is not
reused for footnotes.

## Page-local bounded reflow and convergence

A page-local evaluation is distinct from a global pagination pass. Evaluation
zero fragments the body once using the deterministic incoming-carry reservation
seed and is not a footnote reflow. Every later evaluation applies the preceding
evaluation's ordered set, body cutoff, continuation state, and reservation,
then re-fragments the body; each such evaluation consumes exactly one
`max_footnote_reflows_per_page` unit before fragmentation or allocation.

For evaluation `n`, the sole encoder creates this ordered tuple and applies
domain-separated JCS/SHA-256 using
`typaxis.footnote-page-evaluation/1`:

```text
(
  package/profile/LayoutEpoch/global-pass/page/master/page-start identity,
  body selected-candidate fingerprint and continuation,
  ordered (FootnoteId, FootnoteFlowId, assignment ordinal, first-reference owner),
  ordered per-flow before/after cursor, selected fragments, and carry-out,
  exact reservation
)
```

Allocation IDs, worker order, diagnostics, and trace metadata are excluded.
IDs and arrays have the unique keys stated above before JCS; JCS is not an
ordering algorithm.

A page is `converged` only when two consecutive complete evaluation tuples are
byte-identical and the second evaluation proves that its applied reservation
and body cutoff equal its derived values. Equality of reservation alone,
FootnoteId set alone, an opaque progress hash, or body fragments alone is not
convergence. A match with a non-immediately preceding tuple is an oscillation,
not convergence, and fails with `G6002`. There is no cycle or lowest-cost
fallback for page-local footnote reflow.

The configured maximum `M` is inclusive. At most evaluation zero plus charged
evaluations 1 through `M` may run for a `(global pass, page_index)` pair. If
evaluation `M` is still unstable, the owner emits fatal `G6002` and refuses
before starting, allocating, or fragmenting evaluation `M + 1`; the page is
not added to a materialized state and Display/PDF cannot start. Convergence on
evaluation `M` succeeds. The positive config minimum of one therefore permits
the initial candidate and one confirmation evaluation.

## Existing limits and diagnostic ownership

All maxima are inclusive. No footnote-specific count, fragment, marker,
reservation, carry, or retry limit is added.

| Subject | Existing limit and exact unit | Consume/check owner | Stable code |
| --- | --- | --- | --- |
| each definition, each reference occurrence, and all descendants | the existing `max_ast_nodes` semantic preorder count; a definition or repeated reference is one existing node, never an additional profile charge | strict decoder/syntax iterative precheck before node index, marker map, or FootnoteFlowId allocation | `P1120` |
| each marker buffer and the selected generated overlay | existing `max_text_buffer_bytes` per marker and `max_text_bytes` across parsed/generated text under the established generated-text accounting | generated-text owner before buffer/store allocation | `T2100` / `T2101` |
| each page-local FootnoteId assignment/carry occurrence, one separator record per nonempty footnote page, and every selected definition fragment | one existing per-materialized-state `max_fragments` record; reference markers remain part of their body fragment and definition markers remain part of the first definition fragment | candidate-scoped fragment permit before record/ID allocation; convergence atomically commits only the final candidate to the state budget | `L5110` |
| each body reevaluation after evaluation zero | one `max_footnote_reflows_per_page` unit for that global pass/page pair | pagination work-budget owner before re-fragmentation or candidate allocation | `G6002` |

The effective AST total is the existing complete semantic count; no unique-ID,
registry, or assignment multiplier is added. Total exactly equal to
`max_ast_nodes` succeeds and total max+1 fails before a marker map or flow
registry grows. The selected-state fragment total is existing body/M2 records
plus the assignment/carry, separator, and definition records above. Exact
`max_fragments` succeeds and the next prospective record fails before issue.
The corresponding exact per-marker/aggregate text maxima also succeed, and the
next generated byte fails before allocation. Reflow exact/max+1 behavior is the
evaluation-zero-through-`M` rule above.

Discarded evaluations cannot issue persistent fragment IDs or selected
receipts. Each candidate is independently bounded by the remaining per-state
`max_fragments` capacity, and only the converged candidate commits its exact
count; the reflow maximum bounds aggregate discarded work. Body fragments
retain their existing single charge. A reference marker, definition marker,
assignment receipt, carry receipt, or convergence receipt never double-charges
the semantic node or selected fragment it binds.

Malformed IDs/order/targets and invented wire/style fields are `P1102`;
unsupported placement/content, unreferenced/empty definitions, invalid frame
geometry, indivisible oversize, and unsatisfiable body/first-definition keeps
are `L5100`; known inapplicable styles are `L5101`; selected fragment overflow
is `L5110`; page-local oscillation or reflow exhaustion is `G6002`; and any
receipt, cursor, order, reservation, paint, trace, or manifest contradiction is
`I9190`. Canonical phase order selects the primary error. A failure never
retries under another profile, split policy, marker style, or smaller note set.

## Receipt, Display, PDF, trace, and manifest closure

The trusted chain is single-directional:

```text
FootnoteProfilePreflightReceipt
  -> FootnoteFlowRegistryReceipt
  -> BodyCandidateReceipt + PageFootnoteDiscoveryReceipt
  -> FootnoteFragmentationReceipt + FootnoteReservationReceipt
  -> FootnoteEvaluationReceipt (bounded loop)
  -> FootnoteConvergenceReceipt + FootnoteCarryReceipt(s)
  -> SelectedFootnoteLayoutReceipt
  -> FootnoteDisplayClosureReceipt
  -> frozen PDF observations + trace/manifest closure
```

Preflight binds the exact 1.2 package, immutable profile, catalog/reference
closure, marker/separator/split/frame policy, and admission session. The flow
registry allocates one dense FootnoteFlowId per definition in canonical
FootnoteId owner order and binds its definition NodeId, owning Document catalog
and related body-flow registry, terminal, package fingerprint, and LayoutEpoch.
Caller registration and first-reference order cannot assign FootnoteFlowIds.

Every page receipt binds the selected body candidate, all reference occurrence
owners, deduplicated ordered set, global assignment ordinals, definition-flow
cursors/fragments, exact geometry/reservation, evaluation index/fingerprint,
and incoming/outgoing carries. Only a converged receipt may enter the selected
layout. The selected layout covers every body reference marker, each referenced
definition's logical content exactly once across its fragments, every carry
edge exactly once, and no unreferenced definition.

Canonical Display paint order is page index; existing M2 body commands in
their selected order (including every reference marker); the one separator;
then carried and newly assigned definitions in page ordered-set order, each in
FlowPosition order. A definition marker precedes only the definition's first
fragment. Separator and definition commands are derived from selected receipts,
not caller coordinates. Missing, extra, duplicate, wrong-marker-byte/style,
wrong-rule, wrong-page/order/cursor, or unselected definition paint is `I9190`
before PDF construction.

The frozen PDF graph and serializer retain exact separator path and footnote
glyph/annotation observations from the selected Display. Existing M2 link and
named-destination closure applies to selected definition fragments; footnote
annotation order additionally binds the page, assignment ordinal, FlowPosition,
and selected line rectangle. Trace and built
manifest conditionally record the profile/registry/selected/paint hashes, body
candidate fingerprint, page evaluation count, ordered IDs and assignment
ordinals, FootnoteFlowIds, exact reservation, definition fragment cursors, and
carry edges. The existing ID-sorted `page.footnote_ids` projection remains but
is insufficient by itself. Body-only, page-union-only, Display-only, or
caller-authored trace/manifest evidence cannot authorize publication.

This profile adds no sidenote, endnote, continuation label, PDF structure-tree
note relation, or semantic tagging promise.

## Compatibility and publication gate

At adoption, public capabilities remain byte-for-byte the three implemented
profiles, and passing `--profile typaxis.machine-pdf/footnote-1` remains an
unknown-profile usage error. Portable 1.2 decode/export may contain footnote
wire data; that is not implementation or release support.

MI3-07 may publish `footnote-1` only in one change set after all of the
following pass locally on every documented host:

1. one normal-pipeline descriptor drives public parsing, capability output,
   preflight, receipts, and fixture coverage;
2. zero/one/multiple/repeated references, catalog order distinct from
   first-reference order, unreferenced/empty/missing definitions, split and
   multiple carries, carry-only pages, body/definition keeps, separator paint,
   indivisible oversize, and every accepted definition block/inline/style
   policy (including page reference and link closure) have positive or
   exact-code negative coverage;
3. evaluation convergence, oscillation, exact/max/max+1 reflow, combined
   assignment/fragment exact limits, marker text limits, and long continuation
   progress are checked at the consuming boundary;
4. receipt-tamper tests reject missing, extra, duplicate, wrong-owner/session/
   epoch/flow/order/cursor/page/reservation/evaluation/paint facts through
   Display, frozen PDF observations, trace, and manifest;
5. `samples/machine-package/matrices/m3-footnote.json` contains a combined
   fixture using every advertised footnote policy with the complete M2 domain,
   and descriptor-to-fixture coverage is bidirectional;
6. public E2E, independent PDF validation/raster and text-order checks, two-run
   byte equality, differently named checkout reproducibility, and documented
   host evidence pass; and
7. contract 1.2 and all DocumentPackage Schema bytes remain unchanged, while
   `paragraph-1`, `basic-document-1`, and `table-1` golden negatives still
   reject definitions/references and the default remains `paragraph-1`.

After that gate only, the public profile array becomes exactly
`basic-document-1`, `footnote-1`, `paragraph-1`, `table-1` in canonical ID
order, while the default remains `paragraph-1`. Raw contract 1.0/1.1 is
`P1103` for the explicit profile. A package combining table and footnote
remains rejected by both standalone M3 profiles; a future combined profile
must adopt that domain explicitly.

## Rejected alternatives

- Broadening `basic-document-1` or `table-1`: immutable profiles cannot gain
  footnote acceptance.
- Numbering by first reference: marker text must exist in the initial generated
  overlay, while page discovery is state-dependent; catalog ordinal is already
  canonical and package-derived.
- Painting one definition for every reference or accepting unreferenced
  definitions silently: either breaks exactly-once definition closure.
- Reserving the entire master footnote region on every page: it changes body
  pagination even when the selected page has no note content.
- Converging on reservation or ID-set equality alone, allowing a page-local
  cycle fallback, or starting max+1: each can select a body/definition state
  that was never jointly evaluated.
- Mixing continuation into the body cursor, dropping trailing notes, relaxing
  keeps, or clipping an atomic note: each loses ownership or progress proof.
- Reusing float carry, column balance, layout-pass, or PDF-object limits for
  footnote reflow/assignment: their units do not match the existing footnote,
  AST, fragment, text, and page limits.

## Consequences

MI3-06 and MI3-07 have one exact profile ID, existing wire vocabulary,
definition/reference closure, marker numbering, first-reference assignment,
reservation/separator geometry, split/carry progress, convergence tuple,
inclusive limits, diagnostics, receipt chain, and publication gate. The narrow
definition block subset and fixed visual policy avoid accidental styling or
pagination defaults; richer note content, authored note presentation,
sidenotes/endnotes, footnotes inside footnotes, semantic tagging, and
table-plus-footnote composition require a future contract/profile decision.
