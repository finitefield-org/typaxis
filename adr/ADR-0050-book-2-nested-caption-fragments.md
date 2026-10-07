# ADR-0050: Original nested captions before parent body rows

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0048/0049 to
serial parent captions. Parents with headers or rowspans, definition tables,
remaining body/page forms and full-book/public/human gates remain open.

## Decision

Keep a serial caption content cursor separately from parallel original cell
cursors. It has the next original caption content and any unfinished child-table
cursor. Store it in one additional private continuation slot when a caption exists;
it grants no cell ownership. The existing NESTPOS1 encoding remains unchanged for
parents without captions. Caption position, row-start phase and selected source/
paint remain bound to the retained state and fragment fingerprint.

Evaluate the caption before entering parent body rows. Child fragments preserve
their native captions, cell owners, repeated-header flags, breaks and spacing.
Original paragraphs directly in the parent caption remain caption leaves with no
cell role. Neither captions nor their children acquire a synthetic TD. Caption
children use the full resolved caption width, rather than the width of a body cell.

An incomplete child or authored caption/child break leaves the body unstarted.
Leading, consecutive and terminal breaks retain their physical blank pages,
including the continuation after a final caption break with empty body cells.
On caption completion, remaining capacity can start the original body row. Multiple
caption children and further nested captions use the same recursive search.

A kept final caption content is tentative until original body content starts.
If no body source can start, retry from the original cursor at a strictly smaller
caption cut. This can split the child before its terminal fragment. Empty row
geometry does not fulfill a keep; an entirely empty body creates no successor
requirement. A direct break after a kept caption child reports the child owner.
The parent/caption/child path shares cumulative record/work and spool limits;
failed and discarded trials do not refund budgets or rewind child arenas.

## Verification

Design 28 §14.182 and the progress ledger record actual driver/source-once replay,
exact/one-short budgets, independent PDF text, original columns, child header
artifacts and notes. A rejected kept caption reference and its annotations start
only on page 3. The original Harano fixture is four pages: its caption child ends
on page 3 and parent body starts on page 4. All 238 preceding PDFs and the two
saved VMB table PDFs retain their bytes. No full-book or remaining parent-header/
rowspan support is claimed by these controlled fixtures.
