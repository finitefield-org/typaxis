# ADR-0083: Retain actual header owners through mixed-page placement

## Status

Accepted incrementally under design 28 on 2026-09-10. Mixed pages place their
selected header variants and compare stable geometry. Variant-aware source
closure, glyph/resource closure and private PDF driver integration remain open.

## Decision

Extend the existing mixed-page placement with sparse entries binding each actual
root-header paint fragment to its exact ADR-0080 header association and global
variant item index. Keep the original base flow for ordinary source content and
body-cell child repetitions. Resolve variant global indexes into the correct
body or definition-local stream before placing any fragment. Preserve absent cell
roles on repeated nested captions and keep source consumption separate from paint.

Expose the sparse association next to page geometry; fragment source indexes must
be resolved through it when present. Place line baselines, bounds and block
viewports from that fragment's real measurement. Prepay all header association
entries and conservative growth-copy work before allocation. Keep all lookup,
placement, failed-trial and stable-pass work/records on the shared search ledger.

Factor common list/footnote marker placement into a single-fragment operation.
The legacy whole-page entry point retains the same behavior and budgets. For pages
with variants, resolve each fragment's exact flow before marker lookup. Repeated
list labels remain paint; a repeated header does not issue another definition
marker. Translate fragments, viewports and markers from their own measurement
body/footnote origin to the actual physical page origin. Resolve equation-number
blocks and shapes from the owning flow before deriving number geometry.

Include sparse association identity and item indexes in stable-page geometry
comparison. Unchanged selections reuse the exact catalog references and stabilize
through actual placement. Empty sparse associations preserve the old path and
allocation/work behavior.

Keep the old single-owner table leaf queries guarded. The mixed placer consumes
explicit owner-aware paint queries instead. Reject source closure when any page
contains a variant association with `table_header_variant_source_closure` until
original-unit repetition and actual variant glyph/resource closure are integrated.
A placed or stable page alone grants no PDF authority.

## Evidence and remaining work

Design 28 §14.217 and the progress ledger record controlled and unchanged Harano
fonts, body/footnotes, list-bearing and nested headers, alternating physical widths
and independently moving body/footnote origins. Verification resolves every placed
fragment through its actual flow, checks item/source identity, baseline/bounds,
selected header-relative tops, marker positions and caption repetition. Cumulative
work/record exact limits and one-short failures include selection, stable placement
and the source-closure guard. Full local regressions are recorded in the ledger.

Dedicated new fixtures cover paragraph/list geometry; new block-header PDF or
numbered block-header acceptance is not claimed. Source closure, actual variant
font/glyph/resource closure and catalog construction in the private convergence
driver are still required. `table_repeated_frame_reflow` remains there. Full-book,
remaining page-name scopes, running regions, columns, public profiles/manifests,
managed hosts, scaling and actual author/human acceptance remain open.
