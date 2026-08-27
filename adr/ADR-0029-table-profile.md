# ADR-0029: Table machine-PDF profile

## Status

Accepted on 2026-08-27 as the target contract for the M3 table slice.

This ADR is a decision gate. It does not add a Rust profile identifier, make
the profile selectable, advertise it in current capabilities, or claim layout,
Display, PDF, CLI E2E, or release support. MI3-02 and MI3-03 may use only
crate-private staging entry points. MI3-04 is the sole publication milestone.

| Status axis | At ADR adoption |
| --- | --- |
| contract-defined | Yes: this ADR and `contracts/machine-pdf-capabilities.md` |
| implemented | No: table grid, subflow, and fragmentation owners are pending |
| public CLI E2E | No: `table-1` remains an unknown public profile |
| release-supported | No: the publication gate is MI3-04 |

## Context

The current `typaxis.contract/1.2` DocumentPackage already has a `table` block,
`fixed` and `fraction` columns, separate `head` and `body` row arrays, and cell
`colspan`, `rowspan`, and block arrays. The current portable syntax validator
also fixes source-order leftmost-free grid placement. Neither public immutable
machine-PDF profile accepts a table, and the wire has no table border, padding,
background, alignment, row-break, or header-repeat switch.

M3 therefore needs one immutable profile and one closed interpretation of the
existing bytes before column/grid implementation begins. The design inputs are
docs/25 sections 8, 13.1, and 13.3 plus the existing invariant I-040. Trust,
phase ordering, deterministic output, diagnostics, and publication rules from
ADR-0027 and ADR-0028 remain normative unless this ADR narrows the table
domain.

## Identity, contract, and staging

The adopted identifiers are immutable:

| Item | Identifier |
| --- | --- |
| machine PDF profile | `typaxis.machine-pdf/table-1` |
| required raw DocumentPackage contract | `typaxis.contract/1.2` |
| table profile receipt algorithm | `typaxis.table-profile-receipt/1` |
| resolved column/grid algorithm | `typaxis.table-grid-receipt/1` |
| selected table layout algorithm | `typaxis.table-selected-layout/1` |
| table paint-closure algorithm | `typaxis.table-paint-closure/1` |

`table-1` is an explicit profile and never becomes the default. Omitting
`--profile` continues to resolve to `typaxis.machine-pdf/paragraph-1`.
`basic-document-1` and `paragraph-1` remain closed and continue to reject
tables. Raw 1.0 or 1.1 input requested with `table-1` is `P1103` at
`/contract`; the implementation does not synthesize 1.2 style semantics.

The profile is implementable without changing the current contract or any
DocumentPackage Schema byte. Before MI3-04, the public `MachinePdfProfileId`,
help, dispatch, capability JSON, and normal package commands do not recognize
`table-1`. Private staging must use the exact current 1.2 decoder and typed AST,
not a second table DTO or a hidden public selector.

If MI3-02 or MI3-03 discovers that an additional wire member, style property,
or changed meaning of an existing member is necessary, work stops before that
new shape is implemented. A separate contract-migration task, new contract ID,
versioned Schema, compatibility table, and dependent task-graph edges must be
added before implementation resumes. Mutating 1.2 or treating a new member as
an implementation detail is forbidden.

## Closed accepted wire and style domain

`table-1` contains the complete `basic-document-1` behavior outside tables,
without changing any M2 policy. Its only new content acceptance is a table as a
direct child of the document body. A table inside a list item, figure caption,
another table cell, or another nested flow is `L5100`. Footnotes and every
other M3-or-later feature remain rejected, including page header/footer frames,
multi-column page layout, and floats.

The accepted table wire subset is exact:

| Axis | Accepted by `table-1` |
| --- | --- |
| table placement | direct document-body block only |
| columns | one or more current 1.2 `fixed` or `fraction` column records |
| rows | `head` followed by `body`; at least one row across the two arrays |
| header rows | every row in `head`, as one leading repeat group; no body row is a header |
| cells | current 1.2 NodeId/span, positive `colspan`/`rowspan`, and zero or more paragraph blocks |
| cell paragraph inlines | `text`, `soft_break`, and `hard_break` only |
| table selector properties | `page`, `space_before`, `space_after`, `start_indent`, `end_indent`, `keep_with_next` |
| cell paragraph styles | the existing 1.2 paragraph property/value/applicability rules |
| page/master/resource/PDF domain | exactly the `basic-document-1` domain, plus visual table placement described below |

Table classes participate only in `table(.class)*` selector matching. They are
not copied to rows, cells, or cell paragraphs. `page` accepts only `auto`.
Spacing, logical indents, and `keep_with_next` retain their ADR-0028 meaning;
the indents determine the positive available table inline size. `width` is a
figure property and never means table width. `font_family`, `font_size`,
`line_height`, `text_align`, `width`, and `keep_caption` on a table selector are
known-but-inapplicable `L5101` failures. Cell paragraph rules, rather than a
table default, must supply every text-producing site's required text style.

The table-specific visual policy is fixed profile data, not authored style:

```text
border = none
background = transparent
cell padding = 0
vertical alignment = block-start
border spacing = 0
```

Contract 1.2 has no declarations named `border`, `background`, `padding`,
`cell_padding`, `vertical_align`, `vertical_alignment`, `border_spacing`, or a
table/cell split control. Any such raw declaration is an unknown 1.2 property
and fails with `P1102` at its declaration JSON Pointer even when its value is
`none`, `transparent`, `0`, or `block-start`. No alias is accepted and no value
is silently replaced by the fixed policy. Variable border, padding, alignment,
background, border collapsing/spacing, row minimum height, explicit table
width, and authored split/repeat controls require a future new contract and a
new profile ID.

## Column resolution

All table geometry uses signed `pdf_point_1_65536` integers. A `fixed` column is
the existing `{"kind":"fixed","width":N}` with inclusive
`1 <= N <= 9,007,199,254,740,991`. A `fraction` column is the existing
`{"kind":"fraction","weight":W}` with inclusive `1 <= W <= 65,535`.
Zero, a negative value, a fraction/exponent, a value outside the range, a wrong
member for the tag, or an extra member is `P1102` before table layout.

Let `A` be the positive available inline size after the table's checked logical
start/end indents, `F` the checked sum of fixed widths, and `R = A - F`.
Column widths are resolved once per materialized layout state at the table's
first placement and remain identical on every page of that state.

1. Sum fixed widths and fraction weights with checked wide arithmetic. Overflow,
   `F > A`, or a non-positive available size is terminal `L5100`.
2. With no fraction column, `F` must equal `A`. A positive unassigned residual
   is not stretched, centered, or assigned to a fixed column; it is `L5100`.
3. With fraction columns, `R` must be positive. For every fraction column,
   compute the rational share `R * weight / sum_weight` in checked `i128` and
   round to the nearest fixed-point unit, ties to even.
4. Let `residual = R - sum(rounded shares)`, which may be positive, zero, or
   negative. Add that signed rounding residual to exactly the last fraction
   column in wire column order. No other column receives residual.
5. Every final column width must be positive and the checked sum of all final
   widths must equal `A`. Otherwise resolution fails with `L5100`; it never
   clips, scales, drops, or reorders a column.

A cell's inline size is the checked sum of the final widths from its origin
through its `colspan`. Because padding, gaps, and borders are zero, that sum is
also the exact cell content-frame inline size. The column-resolution receipt
binds `A`, every input kind/value, every pre-residual rounded share, the signed
residual, its last-fraction recipient, and every final width.

## Grid validation and ownership

The syntax/grid owner validates a table before any cell subflow is registered
or laid out. It uses a one-dimensional `remaining_rowspan[column_count]` array;
it must not allocate a `row_count * column_count` matrix.

`head` and `body` are validated in that order as separate sections. For each
section, rows and their cells retain wire order:

1. At a row start, a zero remaining count means the column is free. For each
   cell in source order, its origin is the leftmost free column.
2. `colspan` and `rowspan` are positive. Checked `origin + colspan` must not
   exceed the declared column count, and checked `row_index + rowspan` must not
   exceed the declared row count of that same section. Thus a rowspan never
   crosses the `head`/`body` boundary.
3. Every column in the proposed span rectangle must be free. The validator
   records the same cell owner and remaining row count for each covered column;
   any overlap or out-of-range rectangle is `P1102`.
4. After all source-order cells are placed, every column in the current row
   must be covered exactly once by a new or continuing cell. A hole is `P1102`.
   Then all positive remaining counts are decremented exactly once.
5. At section end every remaining count must be zero. The `head` array is the
   complete contiguous header group, and the `body` array is never repeated.

Table and row/cell NodeIds retain the current typed Document preorder. Columns
have no invented NodeId: their canonical owner key is `(table_node_id,
column_index)`. A cell owner key is its NodeId plus table NodeId, section
(`head` or `body`), row NodeId/ordinal, origin column, `colspan`, and `rowspan`.
Each cell owns exactly one child `FlowId`; that flow binds its containing body
`FlowId`, package fingerprint, `LayoutEpoch`, owner key, content terminal, and
the exact resolved cell frame. Caller registration or worker completion order
cannot affect origins, FlowIds, trace order, or paint order.

## Row bands, splitting, and oversize

Within one materialized state, each validated cell subflow is laid out at its
resolved width into an ordered sequence of indivisible paragraph fragments and
legal break boundaries. A paragraph line and any existing hard keep group are
indivisible. Empty cell content is an already-terminal transparent subflow; it
does not create an implicit line, padding, or minimum row height.

Logical row-band heights are deterministic. Start every band in a section at
zero, then process cells by `(origin_row, origin_column)`. For a cell, compare
its checked natural content extent with the checked sum of the bands covered by
its rowspan. If content has a positive deficit, add the complete deficit to the
last logical row covered by that cell. No deficit is spread or divided. The
result makes every cell rectangle large enough, keeps content at block-start,
and leaves only transparent trailing space when content is shorter.

The complete `head` group is split-prohibited. It is placed once on the first
page that contains this table and repeated in full on each later page that
contains a body row fragment. If the head group cannot fit an otherwise empty
body frame, layout terminates with `L5100`. When body rows exist, a header is
never emitted on a page unless at least one body-row completion or positive row
fragment can follow it; if the header plus the next legal body progress cannot
fit an otherwise empty body frame, that body row is oversize (`L5100`). A
head-only table is placed once and is not repeated.

For a body row whose remaining band fits, the exact remaining extent is one row
fragment. Otherwise construct the finite cut set from the remaining frame
extent and all active cell-fragment endpoints not beyond it. A cut is legal
only when it is positive and does not bisect an indivisible fragment in any
active cell; a terminal cell's transparent tail permits a cut. Choose the
greatest legal cut, with ordinary integer order as the sole tie-break. Every
active cell uses that common block extent, so shorter content has transparent
trailing area and every continued cell cursor is recorded at the same physical
row boundary.

A body cell is split-capable at a cursor exactly when that rule yields a legal
cut before the row completes. It is split-prohibited at that cursor when its
next indivisible fragment crosses every candidate cut. These are derived
states, not authored properties. If an unstarted row cannot make progress in a
nonempty remaining frame, the row is deferred once to the next page. If no
legal positive cut or complete-row placement exists in the usable empty body
frame (after the required header), the row enters terminal oversize `L5100`
exactly once. There is no clipping, forced cut, scale-down, overflow paint,
header suppression, keep relaxation, or same-candidate retry.

A zero-height logical row advances its logical row cursor once and records one
zero-height structural row fragment; it cannot return the same cursor as
`More`. Every successful step strictly advances the physical offset, logical
row, or terminal state.

For a rowspan cell, the selected state carries only its owner, cell-flow cursor,
vertical offset within the spanning rectangle, and remaining declared logical
rows for each covered column. A physical page split retains the current logical
row and increments its row-fragment ordinal. Completing a logical row
decrements the remaining rowspan exactly once; completing the final covered row
requires the cell flow to be terminal. Missing, duplicate, resurrected, or
wrong-owner continuation is `I9190`. Recursive grid snapshots and a second
rowspan limit are forbidden.

The first header occurrence has `repetition_index = 0`; each subsequent table
page increments it by one without gaps. A `HeaderRepetitionReceipt` binds the
original header row/cell/subflow fragments, table owner, selected state,
repetition index, and target page. Repetition is not a cloned AST, does not
issue NodeIds or FlowIds, and cannot change line breaks, text, or geometry.

## Existing limits and diagnostic ownership

All maxima are inclusive. The owner refuses max+1 before the associated object,
array slot, grid entry, FlowId, fragment ID, or paint record is allocated. No
table-specific or synonymous limit field is added.

| Subject | Existing limit and exact unit | Consume/check owner | Stable code |
| --- | --- | --- | --- |
| table, row, cell, descendant semantic nodes, and style declaration/value nodes | existing `max_ast_nodes` count, without double charging rows/cells already in typed Document preorder | strict decoder/syntax iterative precheck before node index or grid work | `P1120` |
| each table column | one additional `max_ast_nodes` unit, keyed by table NodeId and column index because the wire column has no NodeId | strict decoder before column vector growth; syntax/profile preflight rechecks before column/grid receipt allocation | `P1120` |
| selected body row pieces, original header rows, and every repeated header-row occurrence, including a zero-height structural row completion | one `max_fragments` record in one materialized layout state; contained paragraph fragments continue to consume their existing records | shared fragment work-budget owner before row fragment/repetition ID issuance | `L5110` |
| cell `rowspan` | positive `u16` additionally bounded by the number of rows remaining in its declared `head` or `body` section | syntax grid owner before cell-flow registration | `P1102` |

The effective table AST count is the existing complete semantic AST count plus
the number of column records. A Table, TableRow, or TableCell is never charged a
second time merely because it also owns a grid/flow receipt. `max_ast_nodes`
exactly equal to that total succeeds; total max+1 fails before grid allocation.
`max_fragments` exactly equal to the complete selected-state fragment total
succeeds; the next row or header occurrence fails before materialization.
Rowspan uses the declared section row count because that is its semantic unit;
`max_pages`, `max_column_balance_candidates`, or another similar-looking limit
is not reused.

Malformed current-wire tags/ranges, grid overlap/hole/out-of-section rowspan,
and owner-preorder failures are `P1102`. Unsupported table placement/content is
`L5100`; known but inapplicable current style is `L5101`; column arithmetic,
unsplittable header, and row oversize are `L5100`; row-fragment budget is
`L5110`; receipt-chain disagreement is `I9190`. Canonical phase order decides
the primary failure, and a limit failure never retries with fewer rows,
columns, headers, or fragments.

## Receipt, Display, PDF, trace, and manifest closure

The trusted table chain is single-directional:

```text
TableProfilePreflightReceipt
  -> ValidatedTableGridReceipt
  -> CellFlowRegistryReceipt
  -> CellLayoutReceipt + RowBandReceipt
  -> RowFragmentReceipt + RowspanContinuationReceipt
  -> HeaderRepetitionReceipt
  -> SelectedTableLayoutReceipt
  -> TableDisplayClosureReceipt
  -> PDF observations + trace/manifest closure
```

`TableProfilePreflightReceipt` binds the exact 1.2 package, immutable profile,
style-policy fingerprint, and admission session. `ValidatedTableGridReceipt`
binds table owner, available inline size, column input/final widths and residual,
row owners, every cell origin/span, package fingerprint, and `LayoutEpoch`.
`CellFlowRegistryReceipt` adds the complete canonical cell FlowId set and
terminals. Row-band, row-fragment, rowspan, and header receipts bind all
before/after cursors, physical offsets, logical row/fragment ordinals,
repetition indices, target pages, and the same selected state.

The selected table layout covers every declared row and cell flow exactly once
and every materialized body/header occurrence exactly once. Canonical paint
order is page index, table Document preorder, header occurrence before body,
logical row and row-fragment ordinal, cell origin row/column, then the cell
subflow's existing Display order. A spanning cell paints each selected content
fragment once, never once per covered column or row.

The table painter consumes exact selected cell rectangles. With the fixed
`cell padding = 0` and `vertical alignment = block-start`, a cell content origin
equals the spanning rectangle's inline start and block start. Table geometry
emits zero border/background/spacing operations: `border = none` and
`background = transparent` mean the table itself contributes no `Paint`,
`DrawPath`, image, or fill. Ordinary cell text operations remain owned by their
M2 receipts. An extra decoration op, inferred PDF rule/fill, or
missing/extra/wrong-cell/wrong-page/wrong-repetition child op is `I9190` before
publication. The profile adds no tagged-PDF table structure or semantic header
claim.

Trace and manifest facts are derived from the selected receipts, not rebuilt
from Display coordinates. They include the profile/grid/selected-layout hashes,
resolved column widths and residual recipient, cell FlowIds and spans, logical
row fragments, rowspan carries, and header repetitions. Display closure and PDF
observations bind the same facts. Body-only, grid-only, or cloned-header
evidence is incomplete and cannot authorize a built manifest.

## Compatibility and publication gate

At adoption, current public capabilities remain byte-for-byte the MI2 set:
`basic-document-1`, then `paragraph-1`, with `paragraph-1` as default. Portable
1.2 decode/export may represent a table, but neither that fact nor an Accepted
ADR makes `table-1` implemented or public.

MI3-04 may publish `table-1` only in one change set after all of the following
pass locally on every documented host:

1. one normal-pipeline descriptor drives public profile parsing, capability
   output, preflight, receipts, and fixture coverage;
2. fixed/fraction rounding and residual, fixed-only exact width, dense grid,
   colspan/rowspan, split/oversize, repeated header, zero-decoration paint,
   and exact/max/max+1 limits have positive and exact-code negatives;
3. receipt-tamper tests reject missing, extra, wrong-owner, wrong-epoch,
   wrong-cursor, wrong-rowspan, wrong-repetition, wrong-page, and added
   decoration facts across layout, Display, PDF, trace, and manifest;
4. `samples/machine-package/matrices/m3-table.json` contains a combined fixture
   using every advertised table policy together with the complete M2 feature
   set, and descriptor-to-fixture coverage is bidirectional;
5. public E2E, independent PDF validation/raster comparison, two-run byte
   equality, differently named checkout reproducibility, and documented-host
   evidence pass; and
6. contract 1.2 and all DocumentPackage Schema bytes remain unchanged, while
   `paragraph-1` and `basic-document-1` golden negatives still reject table.

Until that gate closes, passing `--profile typaxis.machine-pdf/table-1` is a
usage error, no current capability artifact lists it, and docs must label it
contract-defined only. Publication does not switch the default profile.

## Rejected alternatives

- Broadening `basic-document-1`: immutable profile IDs cannot gain table
  acceptance.
- Adding decoration, row-break, header, or alignment fields to contract 1.2:
  those are wire semantics and require a migration plus new profile.
- Giving residual to the last physical column: when the last column is fixed it
  would mutate a declared fixed width; only the last fraction column receives
  residual.
- Allocating a row-by-column matrix or caller-authored origins: the existing
  leftmost-free one-dimensional validation is bounded and canonical.
- Cloning header AST/subflows, clipping an oversize row, suppressing a header,
  or forcing a cut through an atomic fragment: each loses ownership or silently
  changes content.
- Reusing page, column-balance, or PDF-object limits for table rows/columns:
  their units do not match `max_ast_nodes` or `max_fragments`.

## Consequences

MI3-02 through MI3-04 have one exact profile ID, existing wire vocabulary,
column residual rule, grid ownership, row/rowspan/header progress contract,
zero-decoration paint policy, limit allocation, closure chain, and publication
gate. The intentionally narrow first profile leaves nested tables, rich cell
block kinds, table semantics, and variable visual styling for a new contract
and profile rather than assigning them accidental defaults.
