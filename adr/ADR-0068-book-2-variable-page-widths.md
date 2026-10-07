# ADR-0068: Converge paragraph layout against selected physical page widths

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects ADR-0065–0067 to
the private source-to-PDF driver. Width-dependent tables and block objects,
running regions, columns and the public/full-book acceptance gates remain open.

## Decision

Allow the source-aware frame plan to retain different body and footnote widths
for reachable first, parity and named masters. Measure against the independent
maximum width/height envelopes while retaining the actual physical rectangles.
Keep the strict component frame APIs. Reject width-changing flows containing
tables, raster/vector figures or block math until their own remeasurement exists.

After actual shaping, page selection, placement and source closure, return widths
to original paragraph starts. Rebind them on each fresh shaping pass. A candidate
can proceed only when every selected line uses its physical region's width and
the complete dense assignment equals the previous assignment. A changed generated
page-label flow invalidates the width memo. Width retries clear PDF stability;
page references still require consecutive equal complete PDF candidates.

Unrestricted source-width optimization can cycle: changing a future start's width
changes the preferred earlier line end, which changes the page owning that start.
Detect repeating assignments with Brent's bounded-state cycle detector. Hash the
source fingerprint, ordered paragraph owners/counts and all width slots, charging
the visits. On a cycle, retain the current source-closed paragraph line ends as
layout constraints. Subsequent passes may subdivide those ranges but cannot merge
across the retained boundaries. Keep refinement active until the source changes.

Use the common line-breaking kernel for this constrained search. Validate strictly
increasing, complete original unit ends, legal break opportunities and indivisible
clusters against each fresh itemization. Empty paragraphs use the sole end zero.
Authored mandatory breaks remain mandatory even when absent from retained ends;
retained allowed ends never become authored mandatory breaks. Atomic overhang and
all fit checks remain unchanged. The selected partition can only refine, so there
are finitely many subdivisions for fixed source. Resource ceilings still apply;
this does not guarantee success for infeasible input or within arbitrary caps.
The refined result minimizes demerits among the permitted subdivisions, without
claiming a global optimum over unconstrained joint page/line layouts.

Use `typaxis.refined-width-inline-break/1` with complete `inline_sizes` and
`line_ends` in canonical JCS. Existing fixed/source-width algorithms keep their
canonical bytes. Retained ends and binding views are charged before allocation;
validation, cycle detection and retries consume cumulative work and pass limits.

Independently verify paragraph widths again during physical math finalization.
A component caller cannot paint provisional maximum-envelope widths by bypassing
the driver's feedback condition. Source closure, selected frames and all later
display, tagged structure, navigation and PDF checks remain authoritative.

## Evidence

Design 28 §14.202 and its progress ledger record controlled and unchanged Harano
body, footnote and generated-page-reference PDFs, exact/one-short driver work,
648 exhaustive constrained schedules, malformed boundaries, cluster atomicity,
empty paragraphs, original mandatory breaks and atomic vector overhang.
The Harano footnote fixture reproduces the original two-state cycle and succeeds
with two refinement passes; no text, font or declared region was simplified.

Independent PDF inspection derives advances from the hash-pinned original sfnt
cmap/hmtx/hhea and source declarations, checking source coverage, available body
and note widths, alignment, baselines, markers and final page references. Existing
PDF byte comparisons remain a separate regression gate. These private staging
results do not grant public-profile, full-book, PDF/UA or human acceptance.
