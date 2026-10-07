# ADR-0048: Book-2 child fragments inside original parent cells

## Status

Accepted incrementally under design 28 on 2026-09-10. This connects nested body
tables under parent rows without rowspans, caption/header regions or unresolved
keep chains. Those parent forms, footnote tables, public/full-book acceptance and
the rest of design 28 remain required. This is not complete nested-table support.

## Decision

Keep each nested table as its original measured table. Its child search returns
actual selected height, original source ranges, cell owners, caption placement,
repeated-header roles and forced-break owners. A parent cell holds its own next
content index and, while a child is unfinished, that child's source-bound cursor.
On completion it advances to the next original sibling content. Parallel parent
cells share the available row height; their heights are never added together.

Before/after table spacing is charged at the first/final child fragment only.
If a complete child fits but its trailing spacing does not, retry the same child
cursor under a reduced capacity before accepting any of that trial. A child may
finish and the following child or parent row may start in the remaining space.

An earlier forced break in another parent cell can invalidate an already selected
child fragment. Discard the tentative row, reduce capacity strictly and reevaluate
from the same original parent state. Consume simultaneous break owners once and
preserve consecutive/terminal blank pages. Child source and child paint are
projected separately: repeated headers remain artifacts and do not demand notes
or create another semantic source copy.

Parent and child searches exchange the sole shared record/work allowance, including
on evaluation failure. Retained cursor arrays, searches, source/paint leaf records,
hash buffers and every discarded trial use that allowance. NESTPOS1 binds semantic
child cursor state without arena allocation indexes. NESTFRG1 binds the resulting
source and physical leaf projection with explicit lengths. Smaller parent candidate
capacities use actual selected leaf ends instead of walking padding units.

The source hierarchy cursor from §14.179 skips consumed descendants at the outer
page level and preserves root keep relationships. Original nested Table/Caption/
TR/TH/TD ownership and resolved column widths are retained. Public contracts and
legacy algorithms are unchanged; tables without this nesting retain their PDFs.

## Current boundaries and evidence

Nested content under a parent header/caption, parent rowspans and parent cell keep
chains still report their original nested-table diagnostic. Definition-table
pagination remains guarded. Read-only leaf queries for definition tables use the
shared body/definition item owner rather than indexing the body-only prefix.
That query safety does not grant footnote page placement.

Design 28 §14.180 and the progress ledger record actual child natural/forced breaks,
repeated and internally broken headers, captions, child rowspans, multiple and
parallel children, three wrapper levels, following parent rows, spacing and notes.
Exact/one-short budgets include trial rollback and source-once replay. Independent
PDF checks verify physical-page text, original glyph origins and child artifact
columns. The original Harano font renders a three-page nested fixture. Full-book
and the remaining nested parent forms are not certified by these small inputs.
