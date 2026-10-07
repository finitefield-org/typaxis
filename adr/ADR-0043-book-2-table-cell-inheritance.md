# ADR-0043: Book-2 table-cell style inheritance

## Status

Accepted for implementation under design 28 on 2026-09-10.

## Decision

Book-2 table cells may carry an optional `classes` array. Omission keeps the
existing wire representation. Explicit null is invalid. Legacy contracts reject
any present array, including an empty one, during decode and typed encoding.
Classes retain the existing identifier, ordering and reserved-name rules.

The private `table_cell` selector establishes an inheritance context between the
table and its authored child blocks. It supports `text_align`, `font_family`,
`font_size` and `line_height`; unsupported geometry, keep and page declarations
are rejected, including through `extends`. Child declarations keep the existing
cascade precedence. Caption style remains outside the cell context.

VMB maps resolved left/right/center column alignment to start/end/center in its
horizontal-LTR profile and assigns the corresponding class to original header
and body cells. Cell text uses the ordinary measured line alignment; source text,
cell ownership, widths, math and repeated-header copies remain intact. No padding
characters, synthetic semantic containers or independently positioned text are
introduced. Public-profile publication remains a separate gate.

## Implementation evidence

Private carrier, source inheritance, common line/PDF placement and VMB producer
are implemented with the evidence in design 14.174 and its progress ledger.
Default left-aligned VMB cells omit the field to preserve preceding wire bytes;
left cells in right/center-aligned parents explicitly reset to start. The source
sidecar retains original cell/column provenance and rejects changed classes before
encoding. Actual PDF glyph matrices and CID widths independently verify unequal
columns, unchanged captions, original/repeated headers and a Harano line with real
math. This is not public-profile or full-book acceptance.
