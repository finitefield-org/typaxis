# ADR-0112: Retain partial running-region attachment and merge budgets

## Status

Implemented incrementally under design 28 on 2026-09-22. Related 32 tests,
agent-run CLI 266 tests, font 76 tests, dedicated checks and workspace type checks
pass without failures or ignored tests. Independent verification covers 759 PDFs /
2,896 pages and 58 resource subsets. All 759 PDFs are byte-identical to the preceding
stage. All 84 frozen source hashes and 21 successful logs are recorded in
run-source-correspondence.json. Overall design acceptance remains open.

## Decision

Add a counted page-region join entry point that returns cumulative work and
record observations on both success and failure. The original API delegates to
the same implementation. Preserve its successful output, reservation order,
source identity checks and errors, including empty selected region sets.

The private driver retains the running-region attachment's counters when master
selection, source preparation, line convergence, glyph projection or final merge
fails. Commit failed line-candidate work when the stable callback was not entered;
otherwise commit it once before projection. Observe projection counters before
propagating its result. Retain the existing begun reshape-pass accounting.
Once source preparation succeeds, retain its records before a subsequent work
charge can reject. The enclosing PDF driver receives these observations even
when attachment returns an error.

The existing projection builder reports attempted work, including attempts above
its ceiling. At the command boundary, exhaust only the available work allowance.
Do not copy an over-limit counter into the command ledger and thereby replace
the original projection error. This is a bounded exhaustion policy, not a claim
that every rejected operation executed. No successful-path charge changes.

## Verification

Two tests use controlled TrueType and unchanged original Harano. Sample sixteen
work ceilings over a multi-page header/footer attachment, requiring actual line,
display and join failures. Compare stage errors, cumulative records/work/spool,
begun line passes and absent PDF candidates with the complete driver. Retry on
the same owner without refunding counters. Exact-ceiling attachment must preserve
the successful fingerprint and counters. Empty/unselected-region join tests
also compare counted success and exact one-step-short failure observations.

## Remaining scope

Failed constructors and source/shape/layout internal work or records not exposed
by the stage owner remain unaccounted. This is not complete command allocation
accounting. Public Book /2, whole-book output, controlled performance,
managed-host and author/human acceptance remain open.
