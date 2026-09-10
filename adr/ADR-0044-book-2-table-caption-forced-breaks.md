# ADR-0044: Book-2 source breaks in table captions

## Status

Accepted for implementation under design 28 on 2026-09-10. This connects existing
private Book-2 `page_break` nodes to table-caption pagination. It does not change
public dispatch, legacy profiles or table-cell break semantics.

## Decision

A caption break ends a physical page. Leading, consecutive and trailing breaks
retain their original owners and each advances a page, including blank pages.
Caption traversal therefore tracks the next original content item independently
of vertical offset and row progress: several breaks can share one measured
coordinate. Only the first unconsumed break can end a fragment. Rows and headers
start after that break on the next page, even if the caption remainder and rows
have zero natural height. Repeated headers remain restricted to actual row
continuation pages.

The break index, before/after source cursor and forced owner participate in the
shared search/work/record/canonical-byte budgets and fingerprints. Existing
fragments without caption breaks keep their prior canonical bytes. Candidate
ranking recognizes a forced boundary and still permits an earlier boundary when
same-page footnotes do not fit. Continuation validation and stable replay compare
source progress, not only coordinates. Reusing a stale cursor at the same height
is invalid.

Forced breaks are nonpainting source items. They never receive glyphs or cell
roles. The final body source closure accounts for each original break once via
the selected page's forced owner, including breaks inside table captions. The
ordinary body item index is not incremented for an internally consumed table
break. Pending footnotes may continue across those pages through the existing
joint-page policy.

A keep spanning a forced boundary fails with the existing owner-specific
`KeepAcrossForcedBreak` diagnostic. Nested tables and body/header-cell breaks
retain their explicit unsupported guards until their own parallel-cell/nested
continuation semantics are connected. ADR-0045 subsequently connects body-cell
breaks in tables without rowspans; forced header breaks and spanning tables
remain guarded. The caption connection must not silently
accept or discard those sources. Original full-book/public/PDF-UA/human gates
remain separate.

## Evidence

Design 28 §14.175 and the implementation ledger record 111 integration tests,
94 pagination tests, 112 independently inspected PDFs, and byte equality for
all 102 baseline PDFs. Controlled original-Harano output additionally verifies
seven physical pages, exact Japanese extraction and visible glyph rendering.
The minimal TrueType fixtures prove source/geometry/extraction only: their glyphs
have no visible ink. Neither fixture class establishes full-book acceptance.
