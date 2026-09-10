# ADR-0081: Select table continuations with actual variant header capacity

## Status

Accepted incrementally under design 28 on 2026-09-10. Actual table-fragment
selection and owner-aware paint queries are implemented. Automatic physical-page
variant choice, mixed page/source closure, display resources and PDF integration
remain open.

## Decision

Add a continuation-only selection API accepting the exact header association from
ADR-0080. Validate the base measurement, table index, limits and source cursor.
The original header/caption must already be consumed; initial, terminal and
foreign cursors do not acquire a repeated-header selection.

Keep the original header height for source coordinates, row bands and caption
boundaries. During this one selection, use the supplied variant's actual height
for physical reservation, remaining capacity, body-cell top positions, retries
and reported used height. This applies to common cuts, independent cells, spanning
rows and nested tables. Restore the ordinary height mode on success, no-fit or
error, preserving the search's cumulative work and retained-candidate charges.

Suppress only the old root header paint. Body-cell child tables retain their own
base-owned repeated headers. The result owns its base semantic selection and
borrows the exact variant header, with a fingerprint binding both. Its paint query
attaches the proper measurement owner to every global item index. Root-variant
leaves remain distinct from base-owned semantic leaves and child repetitions.
The semantic cursor/ranges still consume the original base once.

Return a separate selection type and keep the underlying single-owner fragment
private. The existing page/source-closure/PDF pipeline cannot accept this type
until it handles both geometry owners and their actual glyph/resource closure.
This prevents silently interpreting variant item indexes as base line indexes.

Separate header shared set/base charges from its independent variant measurement
and projection bound. A caller ledger cannot hide the independent measurement
behind a maximum. Each search retains exact references to encountered variant
measurements and charges each once. Prepay the header projection bound for every
attempt, including repeated uses of one header object, conservatively covering
distinct header owners without assuming aliasing. Charge registry entries,
lookups, exact-growth copy bounds and result records before allocation. Preserve
larger caller ledgers and charge failed/no-fit attempts as well.

## Evidence and remaining work

Design 28 §14.215 and the progress ledger record controlled and original Harano
body/note fixtures with common cuts, forced breaks, row spans, nested headers and
nested body tables. Both narrow and wide base measurements alternate between
larger and smaller real header variants. Tests verify changed body capacity from
the same source cursor, complete single source consumption, per-leaf measurement
identity, retained child repetitions, restored ordinary selection and exact
cumulative work/record limits.

These are actual table-fragment selections over declared measurement envelopes,
not new physical-page PDFs. The mixed body/footnote page scheduler must choose the
variant for each observed frame, retain the owner association through placement
and source closure, and include its shaped glyphs/resources in display/PDF output.
`table_repeated_frame_reflow` remains on the existing private PDF driver until that
connection is complete. Full-book/scaling/public/manifests/managed-host and actual
author/human acceptance are also still required.
