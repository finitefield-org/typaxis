# ADR-0071: Feed selected physical widths into root-table remeasurement

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects ADR-0070 to actual
private PDFs for root tables whose selected continuations share a parent width.
Different widths within one continued table and table-local block-width feedback
remain explicit follow-up requirements, alongside the full-design release gates.

## Decision

Derive each root table's parent width from the source-closed selected table parts,
the original measurement parent and the actual body/note root-width delta. Use
selected table parts, including zero-height parts, rather than only painted leaves.
Walk body and definition candidates; check original root ownership and demand
coverage. Unreferenced definitions retain original measurement widths. Require all
selected occurrences of one root to agree; differing continuation widths currently
return `PendingRegion("table_continuation_width_reflow")`.

Retain ordered root widths and a physical-match result in the shared width-feedback
owner. Bind them to the fresh exact source flow in the driver. Compare and hash
the complete root assignments alongside paragraph and block assignments, and
remeasure the hierarchy before each subsequent shaping/page pass. Generated-label
changes invalidate the assignments as before. Keep the original work/pass ceilings.

Resolve a placed leaf's root-table membership from its exact original global item
range, preserving the distinction between body and definition-local positions.
Table paragraph widths come from newly resolved cell/caption frames, not a body
width delta or a stale dense paragraph override. Mark these candidates explicitly;
the driver uses the table-frame fallback and does not attach paragraph-only
refinement boundaries to them. A table-width cycle still consumes the declared
pass/work allowance and cannot authorize unstable output.

At math/PDF finalization, independently recompute root widths and reject mismatch.
Apply the guard with explicit table candidates or any varying-width plan containing
tables, including a component caller that has not run driver feedback. Then verify
table paragraphs against their resolved frames and other paragraphs against their
actual page roots. Source closure, original grid/owner identity and resource
binding remain prerequisites, not substitutes for physical-width verification.

The reflow page-plan path now admits tables for this actual verification. Its
source scan charge remains; the strict component plan still rejects differing
widths. Table-local block feedback remains a separate requirement and reports
`table_block_width_reflow` rather than applying an obsolete parent override.

Precharge target/coverage records before allocation. Charge table/candidate/leaf
visits, original-frame lookups, hash/comparison work and final physical verification
to shared budgets. Include the source-region map lookup when measurement lookup
falls through the block registry; its previous block-only depth was insufficient
for first-pass and non-block owner searches.

## Evidence

Design 28 §14.205 and the progress ledger record controlled-font and unchanged
Harano PDFs for three body tables, three demanded definition tables, and a table
continued across equal-width first/even pages followed by a wider table. Each case
retains nested tables, captions, headers, colspan and rowspan, and succeeds at
exact observed work while rejecting one work short. Provisional finalization and
incompatible continuation widths are negative cases.

The independent checker reads physical text matrices and resolves original
paragraph ownership through PDF ParentTree/structure IDs. It compares each
caption/cell/nested paragraph with declared masters, original font advances,
fixed/fraction columns and authored indents. It checks full source text and
right-aligned note markers; displaced or missing text fails. The checker does not
claim independent global line/page optimality or full-book human acceptance.

Private staging PDFs do not establish public-profile, manifest, managed-host,
full-book, PDF/UA or author/human acceptance.
