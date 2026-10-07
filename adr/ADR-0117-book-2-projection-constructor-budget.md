# ADR-0117: Preserve failed body and header projection construction charges

## Status

Implemented and locally verified under design 28 on 2026-09-22. Overall
design acceptance remains open.

## Decision

Add counted staging constructors for body flow, table measurements, repeated
header variants and header catalogs. Existing entry points delegate to the same
implementations. The shared table-measurement kernel reports its accepted local
record reservations on success and failure, including a later canonical-buffer
spool rejection. Its legacy caller retains the existing interface.

Body-flow collection keeps its local record ledger through leaf collection,
named-page checks and marker/reference finishing. Preserve the distinction
between shared historical input and independently retained block records.
Verification order and the original typed errors are unchanged. These two
constructors expose record accounting; previously unmetered internal work is
not presented as measured work.

Header variants and catalogs retain their accepted local work and record
bounds if allocation, geometry/source validation or a later work reservation
fails. Rejected work increments remain uncharged. The existing shared-history
and independent-projection accounting remains intact; storage bounds are
published before allocation. Catalog observations also retain the histories
of headers already verified during traversal.

The ordinary PDF driver and the header-catalog driver recover these
observations before propagating errors. Successful prefix accounting remains
cumulative and is not charged twice.

## Verification

Actual TrueType and unchanged original Harano line graphs exercise header
work/record limits, exact success, empty catalogs, duplicate/reversed entries,
width mismatch, invalid indexes and driver failure before the catalog callback.
Tests compare retained counters with successful owner counters and preserve
original typed errors and fingerprints.

Body/measurement tests use empty content and nested tables, exercise record
reservation boundaries and exact success, and compare constructor observations
with driver observations for named-page conflicts and post-reservation spool
failure. Evidence is recorded in
`workspace/target/vmb-design/20260922/projection-constructor-budget/`.

Workspace all-feature/test checking, 276 Book /2 CLI tests, 104 pagination
tests, 76 font tests and public/legacy CFF diagnostic regressions passed.
Independent validation accepted 759 PDFs / 2,896 pages and 58 subsets; all
759 PDFs were byte-identical to the preceding stage. The correspondence
report binds 123 frozen sources and 16 successful logs.

## Remaining scope

Other preparation owners, source admission/shaping, unmetered internal work
and complete command allocation accounting remain open. This does not publish
Book /2 or establish whole-book, managed-host, controlled performance or
author/human acceptance. User-confirmed missing Speech and SemanticRef data
remains uncreated.
