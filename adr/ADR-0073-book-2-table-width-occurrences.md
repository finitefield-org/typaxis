# ADR-0073: Observe continued table widths by original source positions

## Status

Accepted incrementally under design 28 on 2026-09-10. This supplies source-bound
observations for future heterogeneous-width continuation remeasurement. It does
not authorize such continuations for PDF output or complete the full design.

## Decision

Keep each selected root-table occurrence on its physical body or footnote page.
Do not collapse differing parent widths into a single root-wide assignment.
Require the exact source-closed flow and the issuing mixed-page search, then retain
the original root owner, physical page index, optional definition index and parent
width derived from the original measurement frame and selected region.

Observe semantic leaf ranges separately from repeated placement leaves. Record
paragraphs using original logical-unit ranges, including empty ranges for empty
paragraphs; record block and mandatory-break source owners directly. Logical units
are source scalars and atomic inline items, not UTF-8 byte offsets. A repeated
header/caption is an observation, not a second semantic consumption. Retain empty,
zero-height selected table occurrences even when they have no paint leaves.

The report is immutable and contains no transient leaf indexes, selected-line
ordinals or shaped-item pointers. Bind its digest to the original flow fingerprint,
a distinct algorithm tag, every occurrence and every source piece including its
repeat flag. Generated labels form part of source identity. Content equality can
help later rebinding but does not replace exact issuer/source authorization.

Count occurrences and each occurrence's source pieces before allocating their
vectors. Precharge all retained records, use checked size arithmetic, and charge
counting, selected-part visits, leaf visits and digest folding to the shared search
work budget. Allocate each vector once; repeated exact-one growth must not introduce
quadratic copying outside the work budget. The report exposes cumulative records
and work, with exact-budget success and one-short failure required.

## Consequences and evidence

The current root-wide feedback and finalization paths still reject conflicting
physical widths with `table_continuation_width_reflow`. Per-source cell widths,
horizontal origins and repeated-header layouts must still be remeasured and
converged before this restriction can be removed. This observational API grants
no physical-page or PDF receipt.

Design 28 §14.207 and the progress ledger record body/note continuation tests for
ordinary rows, paragraph splits, repeated headers, mandatory caption breaks and
zero-height tables. Separate table-block tests check original block-owner order.
Unchanged Harano tests distinguish scalar-unit positions from multibyte source
spans. Issuer mismatch and record/work boundaries remain explicit tests. Regression
PDF checks preserve the existing accepted private staging outputs.

Public profiles, manifests, original full-book output, managed-host results and
PDF/UA, author and human acceptance remain outside this increment.
