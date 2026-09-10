# ADR-0070: Reproject source table hierarchies at candidate parent widths

## Status

Accepted incrementally under design 28 on 2026-09-10. This supplies source-bound
table remeasurement for subsequent physical-width feedback. It does not yet enable
varying-width table page masters or authorize candidate geometry for PDF output.

## Decision

Extend the borrowed Book-2 source-width assignments with parent widths for all
original root tables in source event order. Match the exact source-flow owner.
Reject foreign, missing, extra, reordered, duplicate, nested or non-table owners
and candidates wider than the original parent. Empty assignments retain the
existing projection. Frameless paragraph layout rejects table candidates.

Reuse the existing frame traversal and fixed/fraction column resolver. At each
root table, replace only the parent width before its original table indentation
and column calculation. Nested tables inherit newly resolved cells; a colspan
uses all covered columns and rowspan retains its original grid ownership.
Recompute caption, cell, container, list and paragraph frames through the same
source hierarchy. Preserve body/note roots, original source, glyph/resource owners
and rounding rules, including a positive or negative residual on the last fraction.

Retain the complete original measurement projection separately. The selected
projection reports both projections' record bounds. Before allocating the second,
reserve its complete bound alongside the retained original. Prepay a conservative
work bound of 64 times the original record bound times the source-event lookup
depth; this covers frame encoding, repeated linear column passes and logarithmic
owner/cell lookups. Each actual shaping pass pays again from the shared remaining
work. Use the distinct `typaxis.book-2-table-width-frames/1` fingerprint algorithm.

Pass the rebound paragraph frames into the actual source-width and shaping loop.
Glyphs and line ends are recomputed from the exact original flow. Retain original
table, paragraph and region measurements for later physical feedback and audit.
Combined block candidates are checked against their newly resolved table parents.

Physical table-width feedback remains a separate requirement, especially for a
table continued across different masters. Paragraph feedback and math/PDF
finalization explicitly reject any table-width candidates with
`PendingRegion("table_width_reflow")`, even when the caller supplies no varying
page plan. The page-plan `HorizontalReflow` guard remains. Source-closed component
geometry alone cannot convert a candidate into an accepted physical PDF.

## Evidence

Design 28 §14.204 and the progress ledger record real controlled-font and unchanged
Harano shaping in body and footnote tables, two roots with nested tables, captions,
headers, colspan and rowspan. Exact raw-unit column/offset checks cover both signs
of the rounding residual. Narrower frames cause actual additional text lines.
Source text/glyph identity, retained measurement records, exact/one-short work
and reshape passes, invalid binding and provisional-finalization rejection are
checked. Existing PDFs and frozen legacy paths remain regression requirements.

This is private staging remeasurement, not new physical-width table PDFs or
full-book, public-profile, manifest, managed-host or author/human acceptance.
