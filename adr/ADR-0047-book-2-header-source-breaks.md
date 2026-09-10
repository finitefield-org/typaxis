# ADR-0047: Book-2 original header breaks and artifact repetitions

## Status

Accepted under design 28 on 2026-09-10. This extends ADR-0045/0046 to original
header-cell page breaks. Nested/footnote tables and public/full-book acceptance
remain separate unfinished requirements.

## Decision

Consume authored header breaks once while traversing the original header. Use
the same independent cell-content cursors and measured spanning-row bands as the
body. A forced fragment can contain only part of the original header, including
an empty page. Simultaneous breaks consume all originating owners once;
consecutive breaks in one cell advance separate pages.

Represent first-traversal header fragments as original cell slices. Their source
owners retain TH ownership. Once all header source is consumed, repeated full
headers are pagination artifacts. Such repetitions do not execute source breaks,
consume source leaves, create another semantic header or request its notes again.
Break items remain nonpainting items in both original and repeated placements.

The existing full-header height limit remains in force: its natural repetition
must fit the body frame. Without an authored header break, an incomplete original
header or its final prefix that cannot start body content moves to the next page.
An empty body can finish with its original header without reserving the full
header height again. A trailing original break followed by body content starts a
full artifact header on the next page.

Retain original-header completion separately from first table-fragment state.
For these tables, begin at the first header row and zero cumulative extent.
Canonical projection bytes include HEADBRK1 and the source-bound cell state;
arena allocation indexes remain excluded. Temporary header/body traversal state,
source-array copies, merged slices and retries use the shared record/work budgets.
No budget is refunded when an unsuccessful tentative prefix is discarded.

This path is private Book-2 only. Legacy profiles and tables without authored
header breaks preserve their preceding behavior and PDF bytes. Paragraph keep
and table keep conflicts retain their originating diagnostics. No new table style
property, public entry point or VMB producer behavior is introduced here.

## Evidence

Design 28 §14.178 and its progress ledger record simultaneous, leading,
consecutive, trailing and blank headers, header/body rowspans, mixed header/body
breaks, empty bodies, caption breaks, notes, oversize and keep rejection. Direct
replay verifies original source exactly once and exact/one-short shared budgets.
Actual PDFs independently verify physical-page text, glyph column/frame positions
and header artifact scopes. Original-Harano rendering and all preceding PDF byte
comparisons pass. These are controlled small inputs, not full-book acceptance.
