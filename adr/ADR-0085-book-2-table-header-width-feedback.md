# ADR-0085: Keep repeated-header frame observations separate from semantic widths

## Status

Accepted incrementally under design 28 on 2026-09-10. Source-closed variant pages
can supply original-source frame feedback. This does not establish converged
variant PDF output; math terminals and glyph/resource assembly remain open.

## Decision

Extend table occurrence pieces with an explicit separately measured header flag
and, for remeasured reports, the actual variant's (start, width). Keep the original
logical-unit interval and repetition flag. Use each header paint leaf's exact
measurement and global item, converting to that flow's body/definition-local
stream before resolving its selected paragraph line or block. Base-owned child
header copies retain the existing path. Never feed a variant line ordinal to the
base flow.

Precount semantic and repeated pieces, reserve once and retain lookup/projection/
walk/hash work and records on the existing ledger. Fold variant identity and its
measured frame values into report fingerprints. No-variant observations retain
the preceding encoding, counters and output. Reports still match original source
content and are candidates, not placement/PDF authority.

Remeasure the original hierarchy at each physical root parent width. A variant
piece must be a repeat and its actual frame must equal that independently projected
frame. Check this both during source-profile collection and final source-frame
validation. Do not overlay its units onto the original semantic width/origin
assignment. The original header occurrence owns that assignment exactly once;
ordinary base-owned repeats retain the existing consistency rules.

Paragraph frame feedback and its block collection skip sparse variant indexes
before any base lookup; the occurrence report supplies their independent frame
proof. Preserve the base semantic paragraph/block profiles, physical mismatch
flags and complete source-unit coverage. Retained cycle boundaries continue to
come from base semantic lines. Final paragraph validation checks the table source
frames before bypassing separately owned header indexes.

The old single-width paragraph/root-table/block interfaces remain explicitly
rejected for variant-bearing closures. Their one-value representation cannot
represent the separate observations; callers must use paragraph_frame_feedback
and table_width_frames. Math finalization retains its variant gate until terminals,
resources and actual driver convergence can consume these owners safely.

## Evidence and remaining work

Design 28 §14.219 and the progress ledger record sixteen controlled/original Harano
combinations, body/notes, common list/nested headers and narrow/wide base layouts.
Compare raw and remeasured observations, then independently resolve each reported
variant against actual placed metadata and selected line units/frame. Require
original semantic frame assignments to remain intact even when later header
copies use a different width. Retained boundaries and cumulative exact/one-short
budgets include the new feedback work.

These fixtures deliberately retain provisional base body widths, so feedback
reports matches=false. Producing a feedback candidate is not final width
acceptance. Dedicated block/native/vector-header tests and PDF assembly, automatic
catalog generation inside the private driver, remaining page-name scopes, running
regions, columns, full-book/public/manifests/managed-host/scaling and actual author/
human acceptance remain open. table_repeated_frame_reflow is still enforced by
the private driver.
