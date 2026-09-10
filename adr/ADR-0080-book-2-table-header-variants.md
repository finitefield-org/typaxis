# ADR-0080: Bind repeated-header geometry to its actual line variant

## Status

Accepted incrementally under design 28 on 2026-09-10. The component prepares
complete header geometry and checks its capacity. Physical page selection,
repeated-source closure and PDF integration remain open.

## Decision

Require both the semantic base measurement and the header's variant measurement
to use exact line owners from one source-compatible reconstructed set (ADR-0079).
A separately reconstructed measurement with an identical fingerprint is rejected.
Verify measurement limits and original table owner, parent and definition identity.
Retain both measurement references; later verification requires the same pair,
even when another variant has identical geometry and content fingerprints.

Compute the complete header band from the variant's actual rows, excluding the
root caption. Traverse its header cells iteratively in source order. Nested tables
contribute their complete captions, headers and body cells. Nested captions retain
an absent cell owner; forced breaks produce no repeated paint. Use original row
positions and measured child extents, including the variant's changed line count.

Each leaf carries a global item index belonging only to the variant measurement,
relative top, optional original cell owner and original paragraph logical-unit
range or block owner. Do not reinterpret its index in the base measurement or
count repeated leaves as another semantic consumption. Reserve the actual variant
height, with an explicit oversize error when capacity is insufficient.

Before allocating traversal/output vectors, count a conservative bound over the
variant measurement graph. Preserve the reconstructed set and both measurements'
record charges, allowing conservative overcount of overlapping history, or a
larger caller ledger. Charge stack/leaf capacity and owner records before allocation;
reserve each vector once. Count identity checks, graph visits, traversal and fixed-
size fingerprint folds in a bounded work allowance. The fingerprint includes the
ordered set, both measurement fingerprints, limits, table, height and source leaves.

## Evidence and limitations

Design 28 §14.214 and the progress ledger record controlled/Harano body/note
fixtures, including nested captions with forced breaks. Expected paragraph owners
come from authored header JSON, independent of measurement leaf ordinals. Every
original unit is covered exactly once by the candidate; root caption/body owners
are excluded. Width changes produce different real line counts and header heights.
Tests cover exact work/record/capacity boundaries, pair verification, bad table
indexes and same-fingerprint measurements outside the reconstructed set.

The component has no page or PDF authorization. It does not yet select a variant
for each physical continuation, replace the existing search's header reservation,
or connect alternate leaves to repeated-source closure and display assembly.
`table_repeated_frame_reflow` remains until those checks are connected. Block
source addresses are supported by the projection, but these new focused fixtures
exercise paragraph geometry; no block-header PDF or full-book acceptance is claimed.
