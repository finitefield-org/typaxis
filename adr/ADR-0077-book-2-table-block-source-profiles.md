# ADR-0077: Rebind table-local block widths and origins from physical occurrences

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects changing block
parent frames in heterogeneous source tables to the private PDF driver. Different
physical frames for repeated copies of the same block remain open.

## Decision

Extend exact-flow block-width assignments with optional body-relative starts in
the same complete source block order. Require matching counts and explicit widths;
reject combining explicit starts with inherited-table-width mode. Preserve ordinary
width-only callers and frameless rejection. On each actual shaping pass, apply
explicit widths, then verify and install origins in the new frame's existing block
region records. Retain original and inherited frames separately for validation.

Check starts and ends against the original body or footnote-content envelope,
excluding the generated note marker. Source widths remain bounded by original
parent widths. Charge source-event walks and region lookups before use; no new
origin copies are needed in layout because the owned region already contains the
start. Hash each original block owner and start into a separate frame domain after
the width hash. This is provisional geometry, not PDF authorization.

In occurrence feedback, collect ordinary block widths separately from table blocks.
Initialize complete source-order width/origin candidates, then overlay table pieces
with parent frames remeasured at their actual physical continuation widths. Charge
origin and visit vectors before allocation. Match every original block owner;
require one semantic occurrence for each observed block and reject inconsistent
widths or starts among repeated pieces with `table_repeated_frame_reflow`.

The private driver rebinds these explicit starts with the source block widths in
fresh passes, retaining inherited mode for callers without block origins. Compare
all starts, include them in assignment/cycle digests, and charge comparison visits.
Paragraph and block candidates converge together; existing generated-label reset,
consecutive PDF agreement and cumulative work/record/pass limits remain.

Existing figure, raster, SVG, native-math and precomposed-vector kernels use the new
parent frame through their ordinary geometry path. Keep intrinsic dimensions,
original source/resource bindings, alignment, caption text and equation-number gap
requirements. Do not shrink fixed objects to make the new column pass.

Finalization independently remeasures occurrence blocks and compares the complete
parent frame. Fixed page plans with explicit block origins also verify horizontal
starts against their source hierarchy; a one-raw displacement at unchanged width
must fail. This supersedes the `table_block_frame_reflow` guard from ADR-0076 for
single source blocks and repeated blocks whose physical frame agrees.

## Evidence and remaining work

Design 28 §14.211 and the progress ledger record real body/note table continuations
with a 1:2 fractional split, forced caption break and original cell content. Ten
controlled vector/native/PNG/JPEG/SVG cases and eight unchanged-Harano figure cases
converge to actual PDFs, with exact full-driver work and one-short rejection.
An independent checker derives columns from original masters and source declarations
and checks actual block coordinates, fixed sizes and equation numbers.

Component tests cover foreign flow, malformed and missing profiles, incompatible
inheritance, envelope violations, explicit source order, exact work/record budgets
and unchanged-width origin tampering in both finalization modes. The full source
and PDF regression retains previous output bytes.

Conflicting repeated paragraph/block variants, remaining page names and running
regions/columns, other book forms, public/full-book/manifest/managed-host work,
scaling and author/human acceptance are still outstanding. Current charged linear
source lookups and full occurrence projections do not establish full-book scaling.
