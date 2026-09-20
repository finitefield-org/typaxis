# ADR-0107: Retain body line work and begun reshape passes on failure

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes:
257 CLI tests, 76 font tests and all dedicated tests, with zero failures or
ignored tests. Independent checks pass for 759 PDFs and 58 resource subsets;
all 759 PDFs remain byte-identical to the preceding stage. All 72 frozen source
hashes match the executed sources. Overall design acceptance remains open.

## Decision

Add a caller-owned BookV2BodyLineBudget and a budgeted body convergence entry
point. Preserve the existing entry points by delegating with a fresh allowance.
The owner retains consumed line-candidate work, accepted source-width/frame
charges and begun reshape passes on both success and failure. A retry using the
same owner receives only the remaining allowance. The initial break is not a
reshape pass; charge a reshape after its permit is issued and before shaping.
The stable callback reports this invocation's work while the owner is cumulative.

Carry actual consumption out of the existing counted line selector and out of
source-width binding, block widths, root-table widths, block starts and source-unit
starts before propagating errors. Preserve the existing table re-projection
prepayment on failure after acceptance. Rejected prepayments and identity checks
before traversal do not invent consumed work. Successful fingerprints, charges
and output formats retain their existing meaning.

Add BookV2PdfConvergenceBudget to the private driver so callers can preserve its
already-accounted counters across retries. Bind it to effective limits and reject
a changed limit identity before work. The ordinary body branch commits failed
line work and begun passes; a stable callback commits them once before entering
downstream stages. Preserve the original stage error and error source.

## Verification and remaining scope

The three focused tests exercise exact and one-short work caps, a failure during
reshape, initial candidate exhaustion, repeated failures with the same owner,
consumer failure, invalid source bindings/starts, foreign flow identity, block
traversal and table prepayment. They also compare the old and new success APIs,
including PDF bytes and observations, and independently sum driver pre-line work
and failed body work to catch omitted charges at that boundary.

This allowance does not account for shaping backend internals, source admission,
all initial frame construction, allocation bytes, failed header variant seeds or
every downstream failure. It is a continuation of the private driver's existing
counters, not a complete command ledger. Public Book /2, whole-book, performance,
managed-host and author/human acceptance remain open. No public identifiers or
acceptance authority are changed.
