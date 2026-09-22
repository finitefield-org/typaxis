# ADR-0116: Preserve standalone footnote constructor charges

## Status

Accepted incrementally under design 28 on 2026-09-22. Constructor tests,
regression coverage and independent PDF verification passed. Overall design
acceptance remains open.

## Decision

Add counted staging entry points for standalone footnote content and demand,
single-table footnote searches, definition-table demand, one mixed definition
and the mixed definition queue. Existing public entry points delegate to these
implementations. Initial observations retain the greater of the caller's
record prefix and the bound source's record charge, with local work zero.

Keep the existing hierarchy, definition scans and recursive child preparation
inside the observed ledger. Pass accepted charges through content preparation
and demand-owner construction before propagating errors. A single-table
constructor reserves an additional owner record before transferring its ledger
to footnotes; that reservation remains observable if footnote preparation fails.
Successful inactive table searches retain their existing empty ledgers.

Validation order and typed errors remain unchanged, including invalid
definition/root rejection before a depleted record allowance and rejection of
definition tables by flat footnote cursors. Rejected reservations add nothing.
This preserves existing metering, without claiming to meter previously
unaccounted source validation or shaping work.

## Verification

Controlled TrueType and unchanged original Harano fixtures exercise empty
content, ordinary footnotes on nested body tables, nested definition tables and
multiple definitions with a body table. Tests compare legacy and counted
construction, work limits, every record-reservation boundary, exact allowances,
retries, unsupported flat definition-table content and invalid indexes.
Existing complete pagination/PDF regressions cover the resulting successful
searches. Evidence is recorded under
`workspace/target/vmb-design/20260922/footnote-constructor-budget/`.

The two final focused tests, workspace all-features test check, 104 pagination
tests, 76 font tests and public/legacy CFF diagnostics passed. The CLI run was
interrupted after 202 named cases succeeded. Process absence was verified;
1,628 generated files were preserved before running exactly the 72 remaining
cases successfully. Inventory and result reconciliation proves all 274 Book /2
cases passed with no overlap or omission. This is partitioned regression
evidence, not a claim that the interrupted process exited successfully or that
the entire 693-test CLI binary was run.

Independent checks accepted 759 PDFs / 2,896 pages and 58 resource subsets,
including altered-input rejection checks. All 759 PDFs were byte-identical to
stage 252. The correspondence report links 113 unchanged source hashes, 16
successful logs, the interrupted log and exact case partition, preserved
artifact hashes, continuation runner/command hashes and original input hashes.
The unchanged original Harano font passed all 23,060 glyph checks. Observed
runtimes are not controlled performance measurements.

## Remaining scope

Header-catalog construction, body-flow/measurement constructors, source
admission/shaping and complete command allocation accounting remain open.
This does not publish Book /2 or establish whole-book, managed-host, controlled
performance or author/human acceptance. User-confirmed missing Speech and
SemanticRef data is not fabricated.
