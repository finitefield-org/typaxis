# ADR-0049: Keep chains across original nested parent-cell content

## Status

Accepted incrementally under design 28 on 2026-09-10. Extends ADR-0048 to parent
cell keep chains and figure-held nested parents. Parent captions, headers and
rowspans, definition-table pagination and full-book/public acceptance remain open.

## Decision

A parent cell keeps its last legal prefix as a transactional checkpoint. The
checkpoint includes its content/child cursor, original source and painted leaf
lengths, forced-break owners, progress and physical top. If the following content
cannot start, restore that prefix before producing a fragment. Other cells' prior
selections remain intact. A last-item keep has no successor in another cell.

Starting a nonterminal child fragment satisfies the preceding paragraph's keep.
A child's own keep applies to its terminal fragment and the next original sibling.
When that terminal fragment fits but its successor does not, retry the same parent
cell from its original state with a strictly smaller capacity based on the child's
actual leaf end. This allows an earlier child split. Keep chains can span child
endings and further kept paragraphs. A completed non-kept content closes the chain.

Discarded projections and cursor states do not refund record or work allowances;
child search arenas remain append-only. Existing NESTPOS1/NESTFRG1 state and
projection fingerprints describe the accepted state, never temporary arena IDs.
The existing no-keep source and PDF encodings remain unchanged.

A direct page break after a kept paragraph or child reports KeepAcrossForcedBreak
at that predecessor's original owner. A whole parent held by figure keep_caption
is selected only when terminal. Any descendant source break conflicts with that
whole-parent constraint at the parent table owner. An indivisible group exceeding
the full frame reports Oversize; a group failing only the remaining space defers.
No new public table style property or synthetic source/cell owner is introduced.

## Verification

Design 28 §14.181 and its progress ledger record real driver PDFs, independent
physical-page text/column checks, discarded-note/annotation rollback, original
Harano output and exact/one-short source and budget replay. All preceding 216 PDFs
and the two saved original-Harano VMB table PDFs retain their bytes. These small
fixtures certify this keep extension, not the remaining nested parent regions,
original full-book output or author/human acceptance gates.
