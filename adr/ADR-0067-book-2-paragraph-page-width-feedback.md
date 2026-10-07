# ADR-0067: Return physical paragraph widths to original source starts

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0065/0066 with
feedback from actually selected, placed and source-closed pages. Variable-width
master admission and complete private/public driver integration remain open.

## Decision

Compute paragraph width candidates only from the issuing search's exact source
closure and physical geometry. For each original selected paragraph line, derive
available width from its measurement envelope and the selected body/note region's
width difference. Keep original scalar/atom start/end ranges and fill every start
slot covered by that line. Empty paragraphs use their one nonpainting slot.

Retain paragraph owners and exact source-flow content fingerprint, including generated
labels, so candidates can be replayed on a rebuilt but equivalent flow. This replay
still creates fresh exact item bindings through ADR-0066; the feedback object grants
no shaping, page or PDF authority. Changed flow content requires new candidates.

Track semantic and repeated occurrences separately. A repeated header/caption may
revisit the same original units only at the same available width. Every observed
paragraph needs complete semantic coverage. Entire unreferenced definitions remain
explicitly unobserved, with their measurement widths as provisional defaults. Reject
a changed root width in a flow containing tables until column/cell remeasurement is
available; subtracting a root delta does not solve fractional column layout.

Charge result and temporary coverage records before allocation, and charge paragraph,
page, fragment and unit visits to the issuing search's remaining work. Candidate
records and work retain the existing cumulative accounting. Source/sequence identity
and coverage checks remain in force.

Use each selected line's width for body alignment and empty-line bounds. The paragraph
width is a containing envelope; using it for end/center slack incorrectly moves a
narrow provisional line to the envelope's far edge. Require the selected width not
to exceed that envelope. Fixed-width behavior remains byte-compatible.

## Evidence

Design 28 §14.201 and its progress ledger retain actual shape → page → source closure
→ width feedback → fresh shape → page round trips for start/end/center alignment,
unchanged Harano and empty paragraphs. Mixed fixtures cover repeated table headers,
lists, demanded notes and an unreferenced definition. Foreign-search closures and
one-short shared record/work allowances fail. Fixed-width PDFs are regression checked.

This proves paragraph feedback and corrected provisional geometry. It does not lift
HorizontalReflow, solve width-dependent table/block layout, or claim a variable-width
physical-page PDF or full-design completion.
