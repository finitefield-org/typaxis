# ADR-0074: Remeasure table occurrence frames from original hierarchy

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0073 observations
with actual cell/paragraph/block parent frames for each physical occurrence.
Heterogeneous continuation shaping, final placement and PDF acceptance remain open.

## Decision

Project an original root table at one requested parent width without replacing its
retained maximum measurement or another root's candidate. Build complete original
root assignments from source events, substitute the requested root, and invoke the
existing hierarchy/column resolver. Other roots use original measurement widths.
Nested tables inherit new spanning-cell widths. Preserve original source owners,
markers, table/grid declarations and style indents; reuse fixed/fraction rounding
and residual distribution rather than approximating widths by a root delta.

Retain the resulting source projection under an immutable owner tied to the exact
input frame object. Reject non-root, missing or nested owners, exhausted fixed
columns, widening beyond the original parent, foreign verification and insufficient
record/work budgets. The fingerprint includes the requested owner even for no-op
projections. This type exposes provisional geometry, not a physical-page receipt.

Use the original projection's complete record bound, plus a root-list upper bound
and result record, before allocating. Prepay a conservative work bound of 64 times
these records times source-event lookup depth. The shared resolver retains its own
allocation checks. Records include the transient complete projection/root list;
the source measurement remains live. Source-closed pagination prepays these bounds
before the projection attempt, so failed attempts cannot refund search work.

Add a remeasured variant of the occurrence report. For each exact selected root
occurrence, resolve the hierarchy at its observed physical parent width. Attach
paragraph frames to original logical-unit ranges and block parent frames to their
original owners. Mandatory breaks remain nonpainting and carry no frame. Repeated
headers/captions retain distinct occurrence frames while consuming source once.
Keep zero-height table occurrences. Hash remeasured origins/widths under a distinct
report tag. The observations-only API keeps its previous behavior and digest.

## Consequences and evidence

This first implementation builds a temporary complete hierarchy per occurrence,
then retains only the selected source frames in pagination. Its work and storage
are bounded conservatively; full-book scaling acceptance has not been established.
The calculation is real hierarchy remeasurement, but the resulting source frames
must still be rebound through shaping and per-occurrence placement, including
repeated header layouts. Existing heterogeneous-width finalization rejection stays.

Design 28 §14.208 and the progress ledger record controlled/unchanged Harano body
and note cases with original two-column analytic positions, split paragraphs,
repeated headers, caption page breaks and zero-height tables. Separate nested
fixed/fraction/colspan/rowspan tests check both signs of rounding residual, exact
budgets, invalid owners and unrelated-root isolation. Table-local block reports
return inherited parents even when a component explicitly overrides block widths.
No new heterogeneous-continuation PDF or public-profile acceptance is claimed.
