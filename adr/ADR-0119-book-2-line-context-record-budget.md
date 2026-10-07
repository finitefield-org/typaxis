# ADR-0119: Retain line-context, footnote projection and replay record budgets

## Status

Implemented and locally verified under design 28 on 2026-09-22. Overall
design acceptance remains open.

## Decision

The shared selected-line-context kernel exposes its accepted per-paragraph
record prefix. The shared footnote-line projection exposes its input history
and single accepted definition/index/reference storage bound. Existing legacy
entry points delegate without changing validation order or successful results.
Book /2 exposes counted context and footnote projection entry points.

Body-line convergence retains the greatest observed local prefix from completed
line measurements, context capture and footnote projection even when no stable
callback is entered. This is a high-water observation of existing stage-local
reservations, not a sum of released pass storage or complete shape allocation.

The line-variant allowance retains caller history, failed seed-context capture
and prepaid single/set replay storage. Independent captured contexts, shared
rebuilt history and separately allocated replay graphs retain their existing
accounting. Set replay only observes seed histories as it verifies them; it does
not add an unmetered traversal before the existing work checks. Successful seed
and replay owners keep their existing record values and fingerprints.

Ordinary PDF preparation combines a failed body-line observation with the
command's existing independent prefix after retaining its work and begun passes.
Header seed/sibling/replay failures return their cumulative observations to the
existing driver ledger before their stage errors propagate. Successful callbacks
continue using their existing owner charges.

## Verification

Actual multiple-paragraph TrueType/original-Harano sources and unsorted footnote
definitions exercise context success, footnote records, invalid-limit precedence,
all seed capture reservation boundaries and exact record limits. The ordinary
PDF driver's failed line observation is compared with independently run line
preparation. Existing seed/replay tests also compare record observations on
success, partial work failure, foreign sources, exhausted prefixes and retries.
Evidence belongs to
`workspace/target/vmb-design/20260922/line-context-record-budget/`.

The eight focused line/seed/replay tests passed. Final workspace checking,
280 Book /2 CLI tests, 69 layout tests and one layout doc-test, 104 pagination
tests, 76 font tests and public/legacy CFF diagnostic regressions passed.
Independent validation accepted 759 PDFs / 2,896 pages and 58 subsets; every
PDF was byte-identical to the preceding stage. The correspondence report
binds 142 frozen sources and 17 successful logs.

## Remaining scope

This does not account for allocations/work hidden inside failed shape, frame or
line-measurement construction, or establish complete command allocation/spool
accounting. Remaining named-page/column support, public Book /2, original
whole-book output, managed-host/performance and author/visual acceptance stay
open. Missing Speech/SemanticRef remains user-confirmed uncreated.
