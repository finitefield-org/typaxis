# ADR-0111: Retain display and PDF owner observations after failed construction

## Status

Implemented incrementally under design 28 on 2026-09-20. Agent-run CLI 264 tests,
font 76 tests, all dedicated checks and workspace type checks pass without failures
or ignored tests. Independent verification covers 759 PDFs / 2,896 pages and 58
resource subsets. All 759 PDFs are byte-identical to the preceding stage; all 82
frozen source hashes and 20 successful logs are recorded in
run-source-correspondence.json. Overall design acceptance remains open.

## Decision

The private source-to-PDF convergence driver observes the body display builder
after `build_body` and the PDF pipeline after `with_pdf`, before propagating their
errors. These observations include their cumulative search/display prefixes.
Retain the greatest observed prefix once on failure, rather than adding the same
upstream work separately. Preserve record, spool and output high water as well
as work. Recover the search owner's terminal spool even when math finalization
fails before a display can be returned. A successful running-region attachment
also supplies its prefix for a subsequent pipeline failure.

Completed PDF callbacks and width/header retries retain their established
accounting. Observing the pipeline after a completed callback does not charge
the same work again. Keep original stage errors and causes; no successful
candidate is reported for a failed display or PDF.

## Verification

Controlled TrueType and unchanged original Harano tests independently construct
the selected page, math terminals, display and PDF owners. Compare their actual
partial counters with the driver's observations for early/late display failure
and early/middle/late PDF failure. Check all budget dimensions after retries,
and exact-ceiling successful PDF bytes and counters, including a caller-returned
failure value. Full regression and independent PDF evidence are recorded in
the design 28 implementation ledger.

## Remaining scope

This recovers counters already retained by the observed owners. Failed
constructors, partial running-region attachment/join, and nested component work
that is not retained by its owner remain separate gaps. This does not claim
complete command allocation or shaping/source-admission accounting. Public
Book /2, whole-book, performance, managed-host and author/human acceptance remain
open.
