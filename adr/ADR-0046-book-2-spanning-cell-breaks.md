# ADR-0046: Book-2 spanning-cell page breaks over measured row bands

## Status

Accepted under design 28 on 2026-09-10. This extends ADR-0045 to body-cell breaks
in tables containing rowspans. Forced header breaks, nested/footnote tables and
public/full-book acceptance remain separate unfinished requirements.

Subsequent ADR-0047 connects original header breaks and artifact repetitions;
the header-break guard described below records this ADR's initial scope.

## Decision

Retain the original measured row bands as well as each cell's original-content
cursor. A spanning line may extend beyond a neighboring row boundary. The next
row starts in its own column at the measured boundary without waiting for the
entire spanning cell. Within one candidate, retain each cell's painted end so
subsequent content starts after that end. At a physical page boundary, unconsumed
content resumes through its independent source cursor.

The current row's remaining measured height is part of the retained continuation
and its canonical digest. Empty bands and partial padding can advance without
inventing source text. Such band progress does not by itself satisfy a caption
keep_with_next: real row content or an authored source break must start. A kept
caption rolls back if only band progress was possible. Headers are not retained
solely for an attempt that could not start body content.

A later source row may reveal a forced boundary earlier than an already selected
spanning line's end. Discard that tentative candidate and reevaluate its original
cursor under the smaller capacity. Each retry strictly reduces capacity and uses
the existing work/record budgets. No state or paint beyond the new break is
committed. Different cells can still consume simultaneous source breaks once.

When enumerating smaller candidates, move between actual selected cell, caption
and header ends. Padding is not enumerated one fixed-point unit at a time.
Candidate retries, active-cell scans, per-page end/stopped arrays and retained row
state are bounded by the shared budgets before allocation/work. Arena indexes
remain absent from canonical bytes; source content and remaining band state
provide stable replay identity.

The spanning path is private Book-2 only, and is used only where a body-cell break
exists. Tables without spans keep ADR-0045 behavior; tables without cell breaks
and legacy profiles keep their prior algorithms and bytes. Original rowspan,
column and semantic-cell ownership remain intact. Forced header and nested-table
breaks still retain their source-owner diagnostic guards.

## Evidence

Design 28 §14.177 and its progress ledger record actual PDFs for unequal spanning
cells, breaks discovered in later rows, zero-height rows, simultaneous and blank
breaks, repeated headers, caption keep rollback and notes first encountered in a
spanning cell. Independent PDF matrices verify column positions and the following
row's start within the spanning line's band. Work/record exact and one-short tests
include candidate retry. Original-Harano rendering/extraction and all preceding
PDF byte comparisons pass. This does not certify the original full book.
