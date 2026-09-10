# ADR-0084: Close repeated header paint against original logical units

## Status

Accepted incrementally under design 28 on 2026-09-10. Variant-bearing stable pages
can close original-source coverage and resolve each fragment's exact flow. Variant
math terminals, width feedback, glyph/resource closure and private PDF driver
integration remain open.

## Decision

Build a bounded index over original collected items when the search has a header
catalog. Group original paragraph records by their stable source paragraph index,
checking the original owner separately, and block records by original owner. Keep
original logical-unit intervals separately from line ordinals. Prepay both storage
bounds, sorting work, lookups and interval walks; retain these transient proof
charges on the existing search ledger. Do not assume NodeId order is physical
collection order.

For every selected header occurrence, require one ordered placed association for
each actual header leaf. Match exact header references, variant global item,
original owner, body/definition-local item, cell/caption repetition role and the
selected header-relative top. Reject extra, missing, reordered or nonrepeated
copies. Verify the leaf's original-unit interval against its actual selected line,
or its block source against the original block record.

Use a bounded binary lookup and interval walk to map each repeated paragraph line
onto the intersecting original line records. A repeat may split an original line
or join several original lines. Reject gaps, overlaps, outside ranges and empty
intervals without an original empty line. Mark these original records as repeated;
base semantic records must still be consumed exactly once, and unreferenced
source must not acquire repeated paint. Forced breaks remain source events and
are not turned into header paint leaves.

Resolve every placed fragment through its actual flow before source/math checks.
Variant geometry must match actual item bounds and its own physical-origin delta.
Verify vector/native inline receipts and math baselines through that flow, keeping
semantic and repeated counts separate. Expose the verified variant-fragment count
and a bounds-checked fragment-flow resolver on the source closure. This receipt
still does not authorize PDF paint.

Replace the blanket source-closure rejection with this proof. Keep existing
single-measurement width feedback and final math-terminal paths explicitly guarded
for closures with variants until those consumers handle the resolved owners.
Their new boundaries are `table_header_variant_width_feedback` and
`table_header_variant_math_terminals`. No-header closures preserve the existing
path, counters and PDF output.

## Evidence and remaining work

Design 28 §14.218 and the progress ledger record sixteen controlled/unchanged
Harano cases over body/notes, common list headers/nested captions and both narrow
and wide base measurements. Each exercises both physical continuation widths,
actual stable placement and complete source closure with exact cumulative budgets.
Unit tests cover cross-line ranges, exact boundaries, missing/overlapping source,
empty intervals and one-short work limits. Full local and independent PDF
regressions are recorded there.

Dedicated new cases contain paragraph/list header geometry. They do not establish
new native/vector/block-header PDF or glyph/resource acceptance. Actual variant
math terminals, width feedback and font/glyph/resource assembly must consume the
resolved flow before the private driver can construct catalogs and publish such
PDFs. `table_repeated_frame_reflow` remains in that driver. Remaining page-name
scopes, running regions, columns, full-book/public/manifests/managed-host/scaling
and actual author/human acceptance remain open.
