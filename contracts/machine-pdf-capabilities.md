# Machine PDF capability contract

This document records the normative closed machine-PDF profiles adopted by [ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md), [ADR-0028](../adr/ADR-0028-basic-document-profile.md), [ADR-0029](../adr/ADR-0029-table-profile.md), [ADR-0030](../adr/ADR-0030-footnote-profile.md), and [ADR-0031](../adr/ADR-0031-advanced-pagination-profiles.md). The first four are implemented, public, and release-gated. MI3-09 and MI3-10 implement ADR-0031's header/footer and columns targets privately; all three advanced profiles remain non-public and unsupported until MI3-12.

## Status axes

| Profile | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| `paragraph-1` | Yes, ADR-0027 | Yes | Yes | Yes |
| `basic-document-1` | Yes, ADR-0028 | Yes | Yes | Yes |
| `table-1` | Yes, ADR-0029 | Yes | Yes, combined PDF/sidecars | Yes, MI3-04 gate |
| `footnote-1` | Yes, ADR-0030 | Yes: discovery, reflow, carry, paint, and artifact closure | Yes, combined PDF/sidecars | Yes, MI3-07 gate |
| `header-footer-1` | Yes, ADR-0031 on target contract 1.3 | Yes: private MI3-09 vertical slice | No; public profile ID is rejected | No, MI3-12 gate |
| `columns-1` | Yes, ADR-0031 on target contract 1.3 | Yes: private MI3-10 column/balance/artifact vertical slice | No; public profile ID is rejected | No, MI3-12 gate |
| `float-1` | Yes, ADR-0031 on target contract 1.3 | No | No; public profile ID is rejected | No, MI3-12 gate |

Portable DocumentPackage validation, `dump-ast` export, or a staging descriptor does not imply public CLI E2E or release support. A profile becomes release-available only when the same implementation descriptor drives capability output, preflight, combined-fixture evidence, and the documented-host gate.

## Identity and default

- Profile ID: `typaxis.machine-pdf/paragraph-1`
- Current contract 1.2 default profile: `typaxis.machine-pdf/paragraph-1`
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

`typaxis-machine-profile` owns the closed public `PARAGRAPH_1`, `BASIC_DOCUMENT_1`, `FOOTNOTE_1`, and `TABLE_1` descriptors. ADR-0031 reserves three future descriptors but does not permit their public constants or dispatch before MI3-12. The implementation derives all of the following from the registered descriptors rather than maintaining duplicate lists:

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

The current 1.2 artifact publishes these descriptor facts from their sole constant/type owners:

| Fact | Value |
| --- | --- |
| coordinate unit | `pdf_point_1_65536` |
| accepted DocumentPackage contracts | `typaxis.contract/1.0`, `typaxis.contract/1.1`, `typaxis.contract/1.2` |
| `max_resource_roots` | 64 |
| `max_read_candidates` | 131,072 |
| `max_document_package_bytes` default / hard maximum | 134,217,728 / 9,007,199,254,740,991 |
| `max_json_nesting_depth` default / hard maximum | 256 / 256 |
| command-wide maximum diagnostics | 256 |

Exact maxima are accepted; max+1 is rejected before the associated allocation, open, read, work, or ID issuance. Host root/read-candidate and diagnostics caps are fixed security-profile constants, not per-job overrides.

## Public M2 profile: `basic-document-1`

`typaxis.machine-pdf/basic-document-1` is the immutable closed profile adopted by ADR-0028 and published by MI2-08. It is present in current public capabilities and is selected explicitly with `--profile typaxis.machine-pdf/basic-document-1`. Its required raw DocumentPackage contract is `typaxis.contract/1.2`, with versioned Schema `$id` `https://schemas.typaxis.invalid/1.2/document-package.schema.json`. Raw 1.0/1.1 input is rejected at `/contract`; no migration path synthesizes 1.2 properties.

The accepted profile domain is exactly:

| Axis | Accepted by `basic-document-1` |
| --- | --- |
| source | the same one-source, source-0, entry-only closure as `paragraph-1` |
| body/list-item blocks | `paragraph`, `heading`, `list`, `figure`, `page_break` |
| caption blocks | `paragraph`, `heading` |
| inlines | `text`, `anchor`, `reference(format = page)`, `soft_break`, `hard_break`, and non-nested `link` with painted content |
| list | ordered/unordered, checked canonical marker, independent item subflows, nested within existing AST limits |
| style properties | M1 properties plus `space_before`, `space_after`, `start_indent`, `end_indent`, `text_align`, `width`, `keep_with_next`, `keep_caption` |
| page masters | one default body master, `page = auto`, no rules or auxiliary frames |
| images | non-floating PNG with bytes-derived media attestation, required positive computed width, aspect-derived height |
| links | package-local named destination or validated `http`/`https`/`mailto`/`tel` `SafeUri`; one annotation per selected page/line rectangle |
| PDF features | M1 text/destination behavior plus PNG XObjects and internal/external link annotations |

The profile rejects table, footnote, emphasis/strong, non-page references, nested/empty/unpainted links, named-page/master behavior, JPEG/SVG/vector/float, OTF/CFF, outline/tagged PDF, and all M3-or-later domains. It never falls back to `paragraph-1`, reference TSF, another backend, raster substitution, clipped/scaled figure output, or unlinked plain text.

The closed policy summary is:

- list marker bytes are checked decimal plus `.` or U+2022, and the marker stays with the item's first painted line; a marker-only item is rejected;
- each forced page break finalizes the open page and opens the next, so leading, consecutive, and trailing breaks intentionally preserve blank pages and `N` breaks without paint produce `N + 1` pages;
- figures use ties-to-even checked aspect rounding, reject zero/overflow or dimensions larger than an empty body frame, and apply the typed `keep_caption` policy without implicit relaxation;
- links lowercase only the URI scheme, preserve the scheme-specific bytes, require at least one positive-area selected cluster, and union exactly one rectangle per page/line;
- `paragraph-1` remains the `default_profile` throughout contract 1.1 and after the 1.2 migration. `basic-document-1` always requires explicit selection.

The descriptor maps flow/list nodes to `max_ast_nodes`, flow nesting to `max_ast_nesting_depth`, per-marker and selected-overlay bytes to `max_text_buffer_bytes`/`max_text_bytes`, PNG pixels and expanded bytes to `max_image_pixels`/`max_decoded_image_bytes`, link rectangles to `max_fragments`, and annotations/XObjects to `max_pdf_objects`. Exact maxima are accepted and max+1 is refused by the owning phase before work. Stable limit codes are `P1120`, `P1121`, `T2100`, `T2101`, `R7110`, `R7111`, `L5110`, and `G6100` in that order of subjects; no synonymous limit fields are added.

Successful basic-document artifacts bind `typaxis.basic-profile-receipt/1` and `typaxis.basic-flow-registry/1` fingerprints. The preflight receipt hash is carried into trace and manifest layout facts; the registry hash covers the body and every list-item/caption subflow, plus forced-break consumption, list marker groups, figure placement/media, and link rectangles. Missing, extra, wrong-owner/parent/epoch/terminal/target records are `I9190` before publication. Exact property wire values, applicability, keep/oversize behavior, receipt fields, manifest fact names, and the old/new contract compatibility table are normative in ADR-0028 and are derived from the descriptor rather than re-authored by the CLI.

## Public M3 profile: `table-1`

`typaxis.machine-pdf/table-1` is the immutable profile adopted by ADR-0029 and
published by MI3-04. It requires raw `typaxis.contract/1.2`, is the third entry
in current capability JSON, and must be selected explicitly. It does not
broaden either older profile or change the `paragraph-1` default. Portable
decode/export of the pre-existing table wire shape alone is not an
implementation or support claim.

The target inherits the complete `basic-document-1` domain outside tables and
adds only a direct document-body table. Nested/list-item/cell/caption tables are
rejected. A table has at least one current 1.2 column and one `head`-or-`body`
row; cells contain only zero or more paragraphs whose inlines are `text`,
`soft_break`, or `hard_break`. The table selector accepts only `page = auto`,
`space_before`, `space_after`, `start_indent`, `end_indent`, and
`keep_with_next`. Existing text, alignment, figure-width, and caption properties
on a table selector are `L5101`; cell paragraphs use the unchanged paragraph
style contract.

The column contract is closed:

- a fixed column is `{"kind":"fixed","width":N}` in
  `pdf_point_1_65536`, with inclusive `1..9,007,199,254,740,991`;
- a fraction column is `{"kind":"fraction","weight":W}`, with inclusive
  `1..65,535`;
- checked fixed widths are subtracted from the positive indented available
  inline size; a fixed-only table must equal that size exactly;
- fraction shares use checked `i128` rational arithmetic and round to nearest,
  ties to even; the signed rounding residual is assigned only to the last
  fraction column in wire order; and
- all final widths must be positive and their checked sum must equal the
  available inline size. Overflow, unused fixed-only space, non-positive width,
  or oversubscription is terminal `L5100`, never scaling or clipping.

Grid validation processes `head` then `body`, preserving row and cell source
order. Each cell origin is the leftmost column whose one-dimensional remaining-
rowspan count is zero. Positive colspan/rowspan rectangles must be in range,
non-overlapping, cover each row without a hole, and end within their own
declared section row count. `head` is the complete leading repeated-header
group; a rowspan cannot cross into `body`. Columns are owned by table NodeId and
column index; each cell's canonical owner binds table/section/row/origin/span
and exactly one package/epoch/parent-bound child FlowId. Malformed grid shape is
`P1102` before cell layout.

Cell content is at block-start in the zero-padding spanning rectangle. Logical
row bands use the ADR-0029 deficit-to-last-covered-row allocation. A body-row
fragment chooses the greatest positive common cut at or below the available
frame that does not bisect any active cell's indivisible paragraph fragment.
A cell is split-capable or split-prohibited at a cursor solely from those legal
boundaries; there is no authored split property. A row that cannot progress in
a nonempty remainder is deferred once, while failure to progress in an empty
usable frame is one terminal `L5100` oversize transition with no retry, forced
cut, keep relaxation, or header suppression.

The complete header group is split-prohibited, must fit an empty body frame,
and repeats in full before body content on every continuation page. No
header-only continuation page is allowed. Repetitions bind the original header
subflows and source fragments with dense `repetition_index` values starting at
zero; they are not cloned AST/Flow owners. Rowspan continuation carries only
the cell owner/cursor, vertical offset, and one-dimensional remaining declared
row counts, and must advance a row/offset or terminate on every step.

The table-specific visual policy is fixed and emits no decoration:

```text
border = none
background = transparent
cell padding = 0
vertical alignment = block-start
border spacing = 0
```

Contract 1.2 has no table-specific border/background/padding/alignment/split
property. Such a raw name is `P1102` even when it spells the fixed value; it is
never ignored or defaulted. Variable visual policy requires a new contract and
profile. The table adds no tagged-PDF table/header semantics.

Table, row, and cell semantic nodes retain the existing `max_ast_nodes` charge;
each column consumes one additional `max_ast_nodes` unit before vector/grid
allocation because it has no NodeId. Exact max succeeds and max+1 is `P1120`.
Each selected body row piece, original header row, and repeated header-row
occurrence consumes one existing per-state `max_fragments` record before issue;
exact max succeeds and max+1 is `L5110`. Rowspan is bounded by the declared
remaining rows in its own section and invalid range is `P1102`. No synonymous
table count, row-fragment, or rowspan limit is added.

Successful artifacts must bind `typaxis.table-profile-receipt/1` through
`typaxis.table-grid-receipt/1`, canonical cell FlowIds, cell/row-band layouts,
row fragments and rowspan continuations, header repetitions,
`typaxis.table-selected-layout/1`, and
`typaxis.table-paint-closure/1` to Display/PDF observations, trace, and
manifest. Missing, extra, wrong-owner/epoch/cursor/page/repetition or added
decoration facts are `I9190` before publication.

MI3-04 closed the publication gate with bidirectional descriptor/fixture
coverage, a combined all-table-plus-M2 fixture in `m3-table.json`, exact/max/max+1
and receipt-tamper negatives, older-profile table-rejection goldens, external
PDF/raster checks, and deterministic two-run/different-checkout comparison.
Contract 1.2 and current DocumentPackage Schema bytes remain unchanged. A future
implementation needing a new wire/style field requires a separate contract
migration and profile rather than changing this profile or contract in place.

## Public M3 profile: `footnote-1`

`typaxis.machine-pdf/footnote-1` is the immutable contract 1.2 profile adopted
by ADR-0030 and published by MI3-07. It is present in the current descriptor,
CLI, capability artifact, and release gate; `paragraph-1` remains the default.

The target preserves the complete `basic-document-1` content, style, resource,
and PDF domain outside its reference/definition/master-region deltas and adds
`footnote_reference` to body/list-item/caption paragraphs and headings.
It does not compose with `table-1`. Definitions remain in the canonical
Document-owned FootnoteId-sorted catalog and contain one or more paragraph or
heading blocks using the M2 inline subset except nested footnote references.
Each definition must contain positive text-producing content, resolve from at
least one reference, and have no unsupported block/inline/style policy.
Duplicate/missing targets are `P1102`; an unreferenced or empty definition is
`L5100` before flow allocation.

Marker numbering is the one-based canonical definition-catalog ordinal, not
first-reference order. Reference and definition sites use the same shortest
ASCII decimal bytes with no punctuation or whitespace. Every repeated
reference paints its own marker but the definition is assigned and painted
once. A reference marker uses its enclosing paragraph/heading style. The
definition marker uses its definition NodeId and first text-producing
paragraph/heading style, is kept with the first line, and is not repeated on a
continuation. Its first content glyph follows the marker advance plus exactly
one computed `font_size` of layout glue; reference markers add no implicit
spacing or baseline shift.

The sole master has no header/footer/rules and has a non-null footnote maximum
region sharing the body's inline bounds and block end, with a positive height
strictly smaller than the body. A page reservation is zero when it has no note
fragment/carry. Otherwise it is the exact checked sum of a fixed 1 pt separator
band and selected definition fragment extents, at most the master region, and
is subtracted from the body block end. The separator is one full-width black
0.5 pt butt-cap/miter-join solid `StrokePath`, wholly inside that band. No
authored marker, separator, split, continuation, or note-style field exists in
contract 1.2; attempting one is `P1102` rather than a default.

Per selected state, incoming continuations precede newly discovered IDs in
their global first-assignment order. New IDs are discovered from selected body
FlowPosition and typed inline order, deduplicated at first occurrence, and
materialized in page-local first-reference order. Already assigned completed
IDs are not assigned again; candidate ordinals commit only on convergence, so
discarded evaluations cannot leave a gap. The fixed split policy is `allow`:
after reserving one legal minimum-progress fragment for every active carry/new
definition,
remaining capacity is distributed in that order by greatest legal prefix while
preserving later minima. A trailing new reference that cannot reserve its
minimum moves only through a legal body break; incoming carry is never dropped.
Failure to fit the required body line/keep and definition minimum in otherwise
empty maximum frames is terminal `L5100`, with no clipping, forced cut, keep
relaxation, or reordered priority.

Every unfinished definition has a dedicated FootnoteFlowId/source-page/
next-page/cursor carry independent of the body cursor. Each carried cursor must
strictly advance on its next page. A carry-only page holds the body cursor in a
composite page receipt instead of issuing same-position body `More`. Repeated
reference, definition split, and carry therefore have one result: marker sites
may repeat, logical definition content may not, and a page may repeat an ID
only through continuation across different pages. The ID-sorted layout-trace
`page.footnote_ids` projection is retained, while selected receipts separately
preserve first-reference/paint order and assignment ordinals.

Evaluation zero is the uncharged initial body fragmentation. Each later body
fragmentation using the preceding reservation, including a confirmation
evaluation whose reservation is unchanged, consumes one existing
`max_footnote_reflows_per_page` unit for the global-pass/page pair. The
`typaxis.footnote-page-evaluation/1` fingerprint binds the body candidate,
ordered footnote set, every before/after continuation and selected fragment,
and exact reservation plus package/profile/epoch/page identity. Only two
consecutive identical complete tuples are `converged`; a non-adjacent repeat is
an oscillation. With inclusive maximum `M`, evaluations zero through `M` may
run, convergence on `M` succeeds, and an unstable `M` fails with `G6002` before
evaluation `M + 1` starts. Page-local cycle/fallback selection is forbidden.

Definitions and every reference occurrence retain their existing semantic
`max_ast_nodes` units (`P1120`) without a second profile charge. Marker buffers
reuse `max_text_buffer_bytes`/`max_text_bytes` (`T2100`/`T2101`). Each page-local
assignment/carry occurrence, nonempty-page separator record, and selected
definition fragment uses the existing per-state `max_fragments` budget
(`L5110`); markers do not double-charge their containing fragments. Candidate
permits are bounded before allocation, discarded evaluations cannot issue
persistent IDs, and only the converged candidate commits its count. Footnote
carry remains bounded by strict cursor progress, `max_fragments`, and
`max_pages`; float/column/PDF limits are not repurposed.

Successful artifacts must bind `typaxis.footnote-profile-receipt/1` through
`typaxis.footnote-flow-registry/1`, discovery, fragmentation, reservation,
bounded evaluation, convergence and carry receipts,
`typaxis.footnote-selected-layout/1`, and
`typaxis.footnote-paint-closure/1` to the same body state, Display/PDF
observations, trace, and manifest. Paint order is selected body (including
reference markers), the single separator, then carry/new definition fragments
in assignment order. Missing, extra, duplicate, wrong-owner/order/cursor/page/
reservation/paint facts are `I9190` before publication.

MI3-07 closed the publication gate with bidirectional descriptor/fixture
coverage; zero/one/multiple/repeat/unreferenced/empty/split/multi-page carry,
carry-only page, oversize, definition heading/paragraph and complete accepted
inline closure; receipt/paint tamper negatives; the complete-M2
`m3-footnote.json` fixture; external PDF/raster/text-order, two-run
determinism, documented-host evidence, and old-profile rejection. Contract
1.2 and DocumentPackage Schema bytes remain unchanged. Any needed wire/style
field first requires a separate migration and new profile.

## Contract-defined M3 advanced-pagination profiles

ADR-0031 reserves `typaxis.contract/1.3` and these immutable target profiles:

| Full profile ID | Additional closed domain | Publication gate |
| --- | --- | --- |
| `typaxis.machine-pdf/header-footer-1` | custom TrimBox, static master-owned header/footer content, and canonical single or first/left/right selection | MI3-12 |
| `typaxis.machine-pdf/columns-1` | exact left-to-right sequential columns and bounded final-page balance | MI3-12 |
| `typaxis.machine-pdf/float-1` | FIFO nonwrapping direct-body Figure floats across sequential columns/pages, without balance | MI3-12 |

All three inherit the complete `basic-document-1` semantic domain on raw
contract 1.3. They reject tables, footnotes, and each feature belonging to a
different row. The float profile includes unbalanced sequential columns only
because column-boundary queue behavior is part of its required closure. A
neutral package is accepted, but every profile must have a zero-feature
fixture and a complete all-advertised combined fixture before publication.

Contract 1.3 adds required `writing_mode = horizontal-tb` and
`page_progression = ltr` to `page_masters`; required `trim`, nullable
`header_content`, nullable `footer_content`, and nullable `column_layout` to
each master; and required Figure `placement = block|float`. Page-region content
owns a NodeId/SourceSpan and contains only paragraph/heading blocks with
text/soft-break/hard-break inlines. A non-null region requires its existing
rectangle. Portable 1.3 can retain a legacy geometry-only rectangle, but every
advanced profile rejects it and requires exact rectangle/content nullity. A
non-null column layout has count 2 through 65,535,
nonnegative gap, `fill = sequential`, and profile-selected `balance =
last_page|none`. Count one is represented only by null. Margins are derived
from trim/body and are never authored.

The common direction/box policy is physical horizontal LTR. `/MediaBox` is the
selected width/height, `/CropBox` equals MediaBox, and `/TrimBox` is the
checked top-left-to-PDF conversion of trim; BleedBox, ArtBox, Rotate, and
UserUnit are absent. Body lies inside trim. Header and footer share the body
inline bounds, occupy only the corresponding trim margin, and never overlap
body. Custom trim is accepted only by `header-footer-1`; the other profiles
require trim equal to media.

`header-footer-1` accepts either one default master with no rule, or exactly
three first/left/right masters. In the latter form, the default is right and
the only rules are dense source-order first (`first=true`, `parity=any`) then
left (`first=null`, `parity=even`); named pages and footnote frames remain
absent. Each present region has one MasterId-bound source FlowId, restarts from
source start on every selected page, and receives a per-master/kind dense
repetition index. It must reach terminal in one frame. Empty is transparent;
oversize is one `L5100` without carry or retry.

`columns-1` partitions body width with checked `(count-1)*gap`, equal floor
widths, and the entire residual on the last physical column. Nonfinal pages
fill full-height frames by ascending column index. On a nonempty terminal page,
balance starts from `ceil(selected_extent/count)` and evaluates strictly
increasing equal-height targets derived from typed positive rejection deficits.
Candidate exactly at `max_column_balance_candidates` can win; `G6003` is
issued on a repeated candidate fingerprint or before max+1. Empty trailing
columns carry the exact terminal cursor and never issue same-position `More`.

`float-1` accepts only a direct-body Figure `placement=float`. Anchor identity
is body FlowId/FlowPosition/Figure NodeId; enqueue atomically consumes that
body boundary. The unsplittable image-plus-caption must fit the minimum column
width and an empty full-height column before enqueue. FIFO head candidates are
`here`, `top`, `bottom`, then `next_page`; they create a full-column-width,
zero-clearance exclusion band and never side-wrap, bypass, reorder, drop,
scale, clip, or fall back to block. Queue length equal to `max_float_queue` and
carry count equal to `max_float_carry_pages` are accepted; `G6004` is issued
before max+1 enqueue/crossing. Body advance, terminal float placement, or a
bounded page crossing must advance every composite pagination step.

The target capability entry adds `advanced_pagination` only to the new
descriptor objects. It records horizontal-tb/LTR, CropBox/MediaBox/TrimBox,
custom-trim support, single versus first/left/right selection, nullable/1..65535
column range, `forbidden|last_page|none` balance, header/footer support, and the
ordered float candidate classes. Existing descriptor objects omit the member
and remain byte-frozen.

A built advanced artifact requires the same canonical
`advanced_pagination` record in trace and manifest. It binds profile/flow/
selected-layout/paint hashes; dense selected page/master/PDF-box/margin/frame
facts; optional balance result; and FIFO queue-before/placement/carry/
queue-after facts. Old profiles forbid that member. Display and PDF owners
reopen exact frame commands and actual page dictionaries; presentation JSON
cannot issue a receipt.

At ADR adoption none of this target is recognized by public input, help, or
capabilities. Public raw 1.3 is `P1103` and the new profile IDs are usage
errors. MI3-12 must atomically validate/freeze the independent 1.3 registry,
switch every current encoder/Schema/config/artifact identity, register all
three descriptors and normal dispatch, add `G6003`/`G6004`, require the
conditional trace/manifest member, remove private runners, and publish
`m3-all.json`. The default remains `paragraph-1`; full migration details are
normative in ADR-0031 and docs/22.

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
- changing the default profile for a published contract without an explicit new contract migration;
- changing diagnostic code/location/primary-order meaning in a way that alters producer control flow;
- treating a host-availability difference as a different semantic interpretation of the same profile.

`paragraph-1` is never broadened in place. M2 is governed by ADR-0028; the M3
table, footnote, and advanced-pagination slices are governed by ADR-0029,
ADR-0030, and ADR-0031 respectively. Other later capabilities require their
own decision-gate ADR fixing a new profile ID, closed domain, limits,
fallback/oversize behavior, publication semantics, fixtures, and migration
rule before implementation begins.

## Contract and release gating

The public capability artifact and current Schema use `typaxis.contract/1.2`.
Its profile array is exactly `basic-document-1`, `footnote-1`, `paragraph-1`,
then `table-1`, while `default_profile` remains `paragraph-1`. MI2-08 froze the
complete former 1.1 registry and published
`samples/machine-package/matrices/m2-basic.json`; MI3-04 published the table
descriptor and `samples/machine-package/matrices/m3-table.json` without
changing DocumentPackage Schema bytes. MI3-07 subsequently published the
footnote descriptor and `samples/machine-package/matrices/m3-footnote.json`
on the same wire; the default remains `paragraph-1`. Future features may not
broaden any public profile in place.

ADR-0031's contract 1.3 and three profile IDs remain non-public target facts.
MI3-09 and MI3-10 add private header/footer and columns decoders, preflights,
layout/pagination, Display/PDF, manifest, Schema, and fixture gates. Until
MI3-12, all three IDs are absent from the public capability artifact and current
Schema aliases.
At MI3-12 the profile array becomes byte ordered `basic-document-1`,
`columns-1`, `float-1`, `footnote-1`, `header-footer-1`, `paragraph-1`,
`table-1`, while the default remains `paragraph-1`. `paragraph-1` accepts the
neutral 1.3 semantic subset; the other old profiles accept raw 1.2 plus only
the exact neutral 1.3 encoding of their frozen behavior; and the new profiles
are raw-1.3-only. Neutral means full-media trim, null page-region content and
columns, block Figure placement, and the profile's unchanged auxiliary-frame
rules.
