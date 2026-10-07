# ADR-0052: Nested child cursors across original parent rowspans

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects parent rowspans
in the private Book-2 nested-table search. Definition-table page placement and
remaining body/page/full-book/public/human gates remain required.

## Decision

Keep the remaining measured parent row height independently of each original
cell's content index and optional child-table cursor. Visit all cells spanning
the current original row, retaining their current physical ends within a trial.
A cell ending in a later row may place its child beyond the current row band;
advance the band only when the cells ending here finish and their measured minimum
height fits. Preserve zero-height intermediate bands and continuation geometry.

If a later cell or original row exposes an earlier authored break after a child
has already painted beyond it, discard the whole trial and evaluate the original
parent cursor at that strictly smaller capacity. Stop each cell after its first
break in a trial, preserving simultaneous break owners. Child selection, spacing,
keeps, source ownership and repeated-header roles use the existing nested path.
Never refund the cumulative record/work charges or rewind child cursor arenas.

Initial parent header bands use the same original cursors. Completing an original
header at a forced boundary enables its full natural repetition on later body
pages. Repeated headers remain paint only. Include remaining row height in the
whole-header rollback checkpoint so a caption/header retry cannot retain geometry
from a discarded trial. Caption cursors remain separate from real parent cells.

Bind remaining height to nested continuation fingerprints with NESTROW1. Include
the bytes in spool checks and charge temporary end/stopped vectors, row scans,
retry attempts and retained state against the shared allowances. Parent tables
without rowspans retain their previous state encoding and execution path.

## Evidence

Design 28 §14.184 and the progress ledger record 142 integration tests, 99
pagination tests, 49 default table tests and 332 independently checked PDFs.
Fifteen controlled inputs cover natural/forced child splits, either parent spanning
column, later-row breaks, simultaneous breaks, three rows with an empty band,
caption keeps, ordinary and forced parent headers, leading/trailing/consecutive
header breaks, repeated nested captions, empty body and notes. The dedicated
oracle checks original word baselines and column positions, repeated glyph roles
and note annotations in 32 PDFs. Additional source-once/exact/one-short budget
replay covers a caption child above a spanning parent body. Unchanged Harano font
metrics independently predict the Japanese baselines; a real two-page PDF renders
the later-row retry. All 300 preceding PDFs and two saved VMB table PDFs keep their
bytes. These fixtures do not certify original full-book or public release gates.
