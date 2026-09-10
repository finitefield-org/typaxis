# ADR-0061: Retain actual page frames separately from measurement envelopes

## Status

Accepted incrementally under design 28 on 2026-09-10. This extends ADR-0060 to
same-width body and footnote regions with varying heights and vertical origins.
ADR-0062 subsequently connects named body scopes. Horizontal reflow, remaining
named-break/parallel/definition forms, running regions, columns and the full-book/
public/managed-host/author/human gates remain open.

## Decision

Prepare an immutable frame plan borrowing the exact styled body. Unnamed page
selection has three reachable classes: physical page zero, subsequent even
physical pages and subsequent odd physical pages. Query only classes allowed by
the source page limit. Charge their original rule scans and master lookups to the
command's cumulative work budget. Named rules remain in the source query but do
not authorize an unimplemented named source transition.

Retain the three actual rectangles separately from the maximum body and maximum
footnote heights used to measure a fixed-width source flow. A measurement envelope
can combine maxima from different masters and is not a physical page rectangle.
The line owner verifies both the exact styled source and the supplied measurement
body before retaining the plan. Existing component entrypoints retain their
original frame contract, including the single-master note-region validation.
The private driver explicitly selects the plan-aware convergence entrypoint.

Require equal horizontal origins and widths across reachable body frames, and
likewise across reachable note frames. Reject horizontal differences as
HorizontalReflow; do not reuse lines at an unmeasured width. Keep the existing
advanced-content/column guards. Preserve note-region containment validation;
note-free authored frames retain their previous allowance to exceed the media
rectangle. This change does not silently normalize those source rectangles.

Use each actual page's body height for candidate enumeration and ranking, and its
body/note rectangles for simultaneous footnote fit. Scope this geometry to one
page evaluation and restore the component search context on both success and
failure. Retain actual body and declared note rectangles in the selected page;
compare them during stability checks and place body content at the selected Y
origin. The final PDF owner validates the selected stable page against the source
master and source-bound plan, rather than comparing a physical page against a
measurement maximum. Existing per-page media, trim and navigation transforms stay
in force.

Table arenas are measured once at the maximum capacity. When region heights vary,
choose the continuation representation using the smallest selected region, with
the note separator deducted from note capacity. If merged common-cut intervals do
not fit there, retain independent original cell cursors from preparation onward.
Do not rebuild arenas or switch cursor representations between physical pages.
An indivisible item that cannot fit the current empty page still fails; do not
insert arbitrary blank pages to reach a later larger master or skip source.

## Evidence

Design 28 §14.195 and the progress ledger record actual five-page body/note flows,
three-page unequal-cell tables in body and definitions, exact/one-short work,
horizontal-reuse refusal and a too-short first-page failure. An unchanged-Harano
fixture exercises changing body/note origins and capacities with visible Japanese
text. The independent PDF check derives baselines, counts, note bottoms, marker
advances and table columns from original source and font metrics; it rejects
altered source, origins and missing/duplicated pages. These are small controlled
fixtures, not evidence of full-book or public-profile completion.
