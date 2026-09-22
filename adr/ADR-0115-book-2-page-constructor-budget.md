# ADR-0115: Propagate page-search initialization charges to the private driver

## Status

Accepted incrementally under design 28 on 2026-09-22. Focused tests, regression
checks and independent PDF verification passed. Overall design acceptance
remains open.

## Decision

Extend the counted table constructor through body and joint body/definition
contexts. The shared context kernel reserves its storage and traverses source
tables using a borrowed ledger. Book /2 child preparation restores accepted
charges before returning an error and leaves successful inactive entries with
their existing empty ledgers. Hierarchy preparation, completed siblings,
definition scans and partial child preparation therefore share one retained
record/work prefix. The legacy production wrapper uses the same iteration.

Add `prepare_book_v2_table_body_search_counted` and its `_with_headers_counted`
counterpart. Existing public staging entry points delegate to them. Preserve
footnote-context preparation charges, the final demand-owner reservation and
partial header-catalog binding. Empty table/definition work remains zero;
rejected reservations are not counted. These counters describe the existing
accepted charges, not newly metered source verification or shaping internals.

The private PDF driver commits constructor observations before propagating the
original error in ordinary and header searches. Header discovery does the same
while retaining its existing begun-pass count. Successful construction is
accounted by the eventual search owner, so its prefix is not charged twice.

## Verification

Actual TrueType and unchanged original Harano inputs exercise empty contexts,
multiple children, definition-only tables and joint body/multiple-definition
contexts. Work ceilings cover table and footnote preparation; tests also cover
exact work/record ceilings, retained prefixes and cumulative retries. Ordinary
PDF-driver failures are compared with independently prepared constructor
observations and original typed causes.

Separate real header-catalog tests cover partial catalog binding, work and
record ceilings, and retries. Header-discovery failures are compared with the
independently prepared empty catalog and constructor, including begun passes.
Existing complete header PDF regressions continue to cover successful output.

The four focused tests passed. On the frozen final sources, the workspace
all-features test check, 272 Book /2 CLI tests, 104 pagination tests, 76 font
tests and public/legacy CFF diagnostics passed with no failures or ignored
tests. The CLI count is the Book /2 filter, not the entire CLI test binary.
The original Harano font passed all 23,060 glyph checks.

Independent validators accepted 759 PDFs / 2,896 pages and 58 resource subsets,
including altered-input rejection checks. Every PDF remained byte-identical
to stage 251. The evidence directory
`workspace/target/vmb-design/20260922/page-constructor-budget/` contains the
commands, logs and correspondence report linking 104 frozen source hashes,
16 successful logs, 759 actual PDF pairs and original input hashes. Only this
ADR and the two design/progress documents are updated after that verification.
Observed runtimes are not controlled performance measurements.

## Remaining scope

Standalone definition/footnote constructors outside the main page-search path,
header-catalog construction itself, body-flow/measurement construction, source
admission, shaping internals and full command allocation accounting remain open.
This does not publish Book /2 or establish whole-book, managed-host, controlled
performance or author/human acceptance.
