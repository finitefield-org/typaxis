# ADR-0045: Book-2 independent cell continuation at forced page breaks

## Status

Accepted for implementation under design 28 on 2026-09-10. The first connection
covered body cells of tables without rowspans. ADR-0046 subsequently
connects measured spanning-cell bands and retry. It is one part of the original
whole-table/full-book requirement. Forced header breaks, nested tables
and footnote tables remain necessary follow-up work; their guards stay explicit.

## Decision

An authored break in one cell ends the physical page for the parallel row. Each
cell fits complete original items up to the earliest reachable break. A neighbor
whose next line does not fit retains that line for the next page. It must not be
advanced merely because another cell reached a vertical coordinate. Breaks that
parallel cells reach at the same boundary are consumed together. Consecutive
breaks within the same cell each consume a separate page. An authored final break
also preserves the subsequent blank page when no source content remains.

Retain an original-content cursor per cell in a source-bound table search arena.
Candidate records carry an opaque arena index and a digest of the cursor values.
The digest, original measured owner and page progress enter validation and stable
replay. The index is not serialized into canonical bytes: allocation order is not
semantic progress. The existing copyable cursor interface remains intact. State
copies, retained records, per-cell visits, hashing and canonical scratch storage
consume the existing shared finite budgets before allocation or work.

Once all cells in a row finish, the following source row may use remaining page
space. Headers consume capacity only when a row can start and repeat on later
row pages. Caption source remains separate and may share a page with the first
row. A final caption keep_with_next is tentative until some row content fits;
otherwise the candidate moves back to the preceding legal caption boundary.
Forced breaks that directly conflict with a keep report the original owner.

All simultaneous original break items feed exact source closure once. The page
ranking interface retains a representative forced owner; it does not discard the
other owners. Nonpainting breaks never enter glyph placement or cell paint roles.
Caption-only forced pages retain the preceding ADR-0044 behavior.

This path is selected only for private Book-2 tables that actually contain body
cell breaks. Unchanged tables retain their former common-cut algorithm and
canonical bytes. Legacy profiles still reject cell breaks, including in a binary
that enables Book-2 staging. A table containing any rowspan, a break inside its
header or nested table content retains its exact unsupported owner until the
corresponding parallel band/header/nested continuation policy is implemented.
ADR-0046 subsequently removes the body-rowspan guard with measured-band evidence.

## Verification

Design 28 §14.176 and its implementation ledger record actual driver PDFs for
unequal cell line heights, simultaneous and consecutive breaks, empty pages,
caption prefixes and caption breaks, repeated headers, pending notes and caption
keep rollback. Original-Harano output is independently extracted and rendered.
Exact shared work/record budgets and stale equal-height cursors are tested.
These controlled fixtures and old-PDF byte comparisons are not full-book or
public-profile acceptance evidence.
