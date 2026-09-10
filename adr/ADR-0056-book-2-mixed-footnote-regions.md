# ADR-0056: Mixed footnote content in the common region selection

## Status

Accepted incrementally under design 28 on 2026-09-10. ADR-0057 connects shared
required/dependency reservation; ADR-0058 subsequently connects physical page
selection, placement, source closure and private driver PDFs. Remaining body/page
forms and full-book/public/managed-host/author/human acceptance remain required.

## Decision

Extend the existing BookV2FootnoteFragmentSelection and demand selection with the
actual mixed candidate, including its original serial/table parts and next demand
snapshot. Feed it through the same region selection kernel as ordinary content.
A completed definition permits selecting the next pending note in the remaining
capacity; overflow or an authored break ends the candidate region.

The fragment exposes original reference occurrences, first-marker binding, full
continuation, used height and ending spacing. Select references from original
serial ranges or fully covered table leaves; repeated headers create no new source
occurrences. Charge retained reference copies and visited work to the same owner.
Advancing verifies the incoming snapshot and forks the candidate's already derived
demand state, retaining other notes' cursors and avoiding duplicate demands.

Mixed content has no contiguous item slice or consumed range. The staging-only
items and consumed_range accessors now return Result and reject this request for
mixed fragments. Callers inspect mixed parts instead. Existing ordinary page
placement and stability propagate this failure until they handle mixed geometry.
Table continuation and empty root-table ordinals remain independent of serial item
progress; leading breaks and empty tables do not place a definition label.

Sequential selection is not simultaneous reservation. ADR-0057 subsequently
connects required-region and dependency reservation using the full mixed candidate
alternatives. ADR-0058 then connects placement and source closure to actual mixed
source. This region alone still grants no physical page or PDF receipt.

## Evidence

Design 28 §14.190 records the 22-case matrix, repeated region selection with exact
source-once coverage, reference and marker identity, inter-definition spacing,
forced endings, zero-capacity empty roots, stale snapshot refusal and exact/one-short
record/work budgets. Supported PDFs remain byte-identical to the previous stage.
