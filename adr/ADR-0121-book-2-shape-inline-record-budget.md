# ADR-0121: Preserve authored shaping and inline preparation record observations

## Status

Implemented and locally verified under design 28 on 2026-09-22.
Overall design acceptance remains open.

## Decision

The shared authored-text shaping kernel exposes its accepted context and retained
output record counter on success and failure. Legacy shaping delegates to the
same implementation. Book /2 adds `shape_book_v2_authored_text_counted`, which
resets its observation before receipt validation and reports accepted document
reservations before returning a typed failure.

Book /2 adds `prepare_book_v2_inline_items_with_native_context_counted`. The
existing shared paragraph and figure kernels update the caller-visible counter,
including native-math history and accepted units, clusters, anchors and figures.
Rejected reservations are not added. Existing preparation entry points continue
to delegate and preserve successful owner fingerprints.

Body-line convergence commits shape and inline preparation observations before
propagating errors during both initial preparation and every begun reshape.
These stage-local counters combine with frame/line/context history by maximum,
not by summing released pass storage. Seed capture and the PDF drivers recover
that high-water observation through the existing paths. Replay continues to use
its prepaid rebuild bound without charging overlapping observations again.

## Verification

Evidence is stored under
`workspace/target/vmb-design/20260922/shape-inline-record-budget/`.
Actual TrueType and unchanged original Harano fixtures exercise successful
fingerprints, invalid context/epoch precedence, missing glyphs after an earlier
paragraph, partial output limits and exact capacity, and figure geometry failure
after retained text plus hard breaks. Body/seed/driver failures and retries must
preserve the accepted prefix without inventing candidate work or begun passes.
All 22 focused body-budget tests passed. Final workspace checking, 284 Book /2
CLI tests, 27 shaping tests plus one doc-test, 69 layout tests plus one doc-test,
104 pagination tests, 76 font tests and public/legacy CFF diagnostics passed.
Independent verification accepted 759 PDFs / 2,896 pages and 58 subsets; all
759 PDFs were byte-identical to stage 257. The correspondence report binds
156 frozen sources and 18 successful logs.

## Remaining scope

Font-instance construction, backend temporary allocations, paragraph context
buffers and internal shaping work are not newly measured. Page-region shaping
and preparation still need counted propagation through their own budget owner.
Complete command allocation/spool/work accounting, remaining named-page/column
support, public Book /2, original whole-book output, managed-host/performance and
author/visual acceptance remain open. Speech/SemanticRef is user-confirmed
uncreated.
