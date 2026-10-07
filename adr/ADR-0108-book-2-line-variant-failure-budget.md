# ADR-0108: Retain failed line-variant convergence and context capture

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes:
259 CLI tests, 76 font tests and all dedicated tests, with zero failures or ignored
tests. Independent checks pass for 759 PDFs and 58 resource subsets. All 759 PDFs
remain byte-identical to the preceding stage; all 75 frozen source hashes match.
The common PDF verifier was interrupted once and passed on the preserved-log
retry. Overall design acceptance remains open.

## Decision

Extend the body line allowance to the variant-seed boundary using a separate
caller-owned BookV2LineVariantBudget. Preserve accepted convergence work, begun
reshape passes and accepted context-capture charges before returning any error.
A retry uses the owner's remaining allowance. Keep successful seed observations
local to that invocation so cumulative counters are not charged twice.

Keep the existing seed and sibling APIs as fresh-budget entry points. Add
budgeted preparation for both the base source and siblings at new source widths.
Preserve existing flow identity and preallocation checks. A preflight rejection
does not consume work; a context-capacity failure after convergence retains the
accepted convergence and capture work. Existing conservative capture prepayments
retain their meaning and do not claim to be measured allocation bytes.

Connect both the private PDF driver's header base and the header catalog's
sibling loop to these owners. Commit consumed work and begun passes before
propagating the original stage error; update the retained seed record count only
when a seed exists. Successful fingerprints, record counts, work and PDF output
must remain unchanged.

## Verification and remaining scope

Focused tests cover controlled TrueType and unchanged original Harano: successful
API equivalence, exact and one-short capture work, candidate exhaustion, repeated
success and failure on one owner, context-record rejection after convergence,
preflight record rejection, sibling equivalence and foreign/malformed widths.
The existing automatic header driver regression passes, including original
Harano. Independent verification covers 2,896 pages, with all 759 preceding PDF
pairs byte-identical. The interrupted first common-verifier log is preserved;
the completed retry is the acceptance evidence.

Replay/rebuild failures, initial frame construction, shaping internals, source
admission and other failed downstream phases remain outside a complete command
ledger. Public Book /2, whole-book, managed-host, performance and author/human
acceptance remain open. This change grants no new public acceptance authority.
