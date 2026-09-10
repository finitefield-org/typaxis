# ADR-0059: Retain natural cell continuations when common cuts cannot fit

## Status

Accepted incrementally under design 28 on 2026-09-10. Unequal natural cell line
heights now reach actual private pages/PDFs even when no common vertical cut fits
an empty page. Full-book and public acceptance remain incomplete.

## Decision

The existing shared-boundary table search remains in use wherever its merged
blocked intervals can fit the declared maximum region, including repeated header
height. For successor inputs, if a body's merged blocked interval exceeds that
capacity (or meets it at a closed end), prepare the existing independent-cell
cursor path. This decision is source- and geometry-bound at preparation, before
any cursor is issued. Do not switch arenas or reinterpret a live common-cut cursor.

Reuse the original cell source, paragraph keeps, caption/header behavior, spanning
row geometry, fragment fingerprints, shared work budget and exact source closure.
The extra bounded scan is charged to the same owner. Whole-table keeps and genuinely
oversize source items retain their refusal. Frozen inputs do not opt into this
path; neither their preparation charges nor refusal policy change.

Natural line heights from different fonts can overlap to block every common cut
across a row although each cell contains individually fitting lines. The original
Harano fixture exposed this at 48pt. Enlarging that fixture to 80pt demonstrated
other functionality, but was not a solution to unequal-cell pagination. Retain both
input geometries now, with the original font and the same 48pt footnote region.

## Evidence

Design 28 §14.193 records flat/nested/header/caption/rowspan cases at 17/23pt line
heights, paragraph-keep and oversize refusals, exact/one-short page budgets, and
independent geometry/source checks over actual PDFs. The spanning case preserves
the existing source-order row-deficit allocation, including trailing nonpainting
row geometry; it does not shrink a measured band after its text ends. All preceding
384 PDFs and both saved original-Harano VMB table outputs remain byte-identical.
