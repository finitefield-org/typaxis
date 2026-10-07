# ADR-0053: Source-bound definition-table continuation and retained demands

## Status

Accepted incrementally under design 28 on 2026-09-10. Adds a source-only table and
note-demand component. Design §14.187–188 and ADR-0054 subsequently add mixed
definition source candidates; ADR-0055 subsequently retains their continuations
in the common demand queue. Joint region fitting, page placement and the
full-book/public/human gates remain required.

## Decision

Bind an actual definition-table cursor and a branchable note-demand snapshot in
one private Book-2 search owner. Acquire the containing note from actual ordinary
body reference items, never from the mere presence of a definition. Source table
selection and demand transitions transfer the same remaining record/work budget,
including failed evaluations. Do not flatten parallel cells through the ordinary
footnote item kernel.

Only original semantic table ranges can demand another definition. Translate them
to local definition indexes and require all items carrying a reference's glyph
clusters before retaining that occurrence. Process each source occurrence once;
repeated headers do not contribute semantic ranges. A partial reference rejects
the provisional candidate. Fork the demand snapshot before applying a successful
transition, so discarding a candidate leaves the source state unchanged.

Completion of the selected table is distinct from completion of its definition.
Retain a pending definition when ordinary following items or a later root table
remain, including a zero-height table sharing the same leaf endpoint. Complete the
current definition before processing a retained terminal self-reference, so it
cannot requeue itself. Validate source flow, definition, actual root table and
local continuation boundary. A request that skips an unconsumed prefix is invalid.
Bind selections to the exact search/state identity; another candidate branch or
another search owner cannot reuse that selection.

This component starts at its table's actual definition boundary. Mixed candidates
for preceding/following serial content are covered by design §14.187–188 and
ADR-0054; ADR-0055 adds the common queue continuation. Reserving first fragments
across notes, dependency backtracking,
separator/label/header placement and stable source closure belong to the ongoing
common footnote integration. Its page-level unsupported guard remains active.

## Evidence

Design 28 §14.186 records early/late and parallel cell references, repeated headers,
a terminal self-reference, unconsumed prefix rejection, an ordinary suffix and an
empty trailing table. Tests verify source-once traversal, speculative branch
isolation, completion status, missing body demands, wrong owner/branch rejection,
nonrefundable zero-capacity trials and exact/one-short record/work limits. The full
144-test integration and 332 independent existing PDFs pass; those PDFs and both
saved VMB original-Harano table PDFs retain their preceding bytes. No new physical
footnote-table PDF success is claimed by these source-state tests.
