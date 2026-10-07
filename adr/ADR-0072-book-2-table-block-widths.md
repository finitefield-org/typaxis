# ADR-0072: Inherit remeasured table parents for block geometry

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0071 table
remeasurement to table-local figures and display math. Different physical widths
within one table continuation and the remaining full-design gates remain open.

## Decision

Let the source-width assignment explicitly request inherited table-block frames.
The driver uses this mode together with complete original block-owner assignments
and root-table widths. Traverse the exact source events while applying block
assignments, tracking nesting depth and the enclosing root table without allocation.
For table-local blocks, use the newly projected parent width; for ordinary blocks,
use the supplied width. Validate every original block owner in source event order,
including entries whose old width is superseded by their new table frame.

Keep the current explicit-width mode for component callers. Preserve the frame
after table/cell/container projection and before any explicit block override.
Expose this inherited frame separately from both the original maximum measurement
frame and the final block frame. The frame fingerprint includes the effective
width, so stale numeric observations do not replace the newly resolved cell width.

Physical block feedback uses exact source-item table membership. For table-local
blocks, return the inherited frame width; the independently collected root-table
targets still decide whether the table matches its physical page. For ordinary
blocks, retain the original root-delta calculation. Finalization compares actual
block parents to inherited table parents, in addition to independently checking
root-table physical widths and source closure. A component's explicit narrower
override cannot become accepted merely because its root table already matches.

Retain declared raster dimensions, SVG scale, native computations, original glyphs,
caption source and number-gap requirements. Remeasure alignment and equation-number
geometry at the new parent. Native copied fixtures keep distinct original source
ranges and identity mappings. Existing source/resource validators remain intact.

Reuse charged source-event visits and retained block-frame records. Nesting tracking
uses checked arithmetic. Precharge inherited-frame lookup depth, including its
fallback to the region map, through the existing shared work budget. Exact/one-short
work behavior remains required. No additional public profile or receipt is issued.

## Evidence

Design 28 §14.206 and the progress ledger record controlled-font and unchanged
Harano body/note PDFs with nested tables, a spanning parent cell, two fixed columns,
vector figures/captions/numbered formulas, centered native display math, PNG, JPEG
and ordinary SVG. Original block assignments still bind all owners; explicit versus
inherited component behavior and finalization mismatch are tested separately.

The independent block checker verifies the declared two-table column topology,
derives its 40pt parent offset from fixed columns, then checks physical block and
equation-number placement, fixed sizes and native centering from original font
metrics and actual paint glyph extents. Displaced/missing blocks are rejected.
Existing accepted PDFs and legacy paths remain regression requirements.

This is private staging table-block output, not full-book, public-profile,
manifest, managed-host, PDF/UA or author/human acceptance.
