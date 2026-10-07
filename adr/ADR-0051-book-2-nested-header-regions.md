# ADR-0051: Original nested headers and detached repeated caption leaves

## Status

Accepted incrementally under design 28 on 2026-09-10. Connects parent headers in
the existing nested caption/body search. Parent rowspans, definition tables and
the remaining body/page/full-book/public/human gates remain required.

## Decision

Start a nested parent's original header at row zero, after any original caption.
Use its original content/child cursors for authored breaks. A natural header prefix
must complete and start body source in the same fragment; otherwise restore the
whole tentative header to its source-bound checkpoint. Retain an earlier legal
caption prefix, or retry a kept caption at a smaller cut. Authored header/child
breaks can end an original header prefix without starting body. Completing such a
header enables the full repeated header on subsequent body pages.

Repeat the original header at its full natural geometry. Traverse original nested
caption/cell contents, skip source break commands and add paint leaves only. Do not
reexecute a child's forced breaks, demand notes again, or reconsume its semantic
source. Preserve original child cell owners and resolved columns. Direct caption
leaves have no cell owner even when a containing parent header repeats them.

The placed page retains sparse indexes for repeated leaves without cells. A single
linear iterator exposes each fragment's independent cell owner and repeated flag.
Existing cell-role allocations/encodings remain unchanged where no detached copies
exist. Source closure, math terminals, equation labels, text, images, anchors,
nonpainting lines and list/note marker projection consume that flag. Display
fingerprints use their existing repetition fields; PDF scopes and navigation use
the explicit flag instead of inferring repetition from cell presence. Repeated
captions become Pagination/Header artifacts, never invented TH/TD sources or new
anchor destinations.

Header checkpoint cells, repeated leaf retention and sparse indexes share the
existing cumulative record/work allowances. Failed or discarded trials never
refund work or rewind child arenas. Stable geometry compares detached repetition
indexes as well as cell roles. Original full header height still obeys the common
TableHeaderOversize limit. Parent rowspans remain guarded by original ownership.

## Evidence

Design 28 §14.183 and the progress ledger record 139 integration tests, 99 pagination
tests and 300 independently checked PDFs. Controlled fixtures cover nested body
under a parent header, child tables/captions/deeper nesting inside headers, forced
parent/child prefixes, caption keep rollback, notes, empty body and detached image,
anchor and nonpainting caption copies. Dedicated checks verify actual glyphs,
original columns, header height reserved above body and nonduplicated annotations.
Exact/one-short budgets and source-once replay cover header rollback and forced
prefixes. Original Harano renders a two-page repeated nested-caption header. The
preceding 272 PDFs and both saved VMB table PDFs retain their bytes. These controlled
fixtures do not certify the original full book or the remaining design gates.
