# ADR-0109: Retain failed single and multiple line-graph replay work

## Status

Accepted incrementally under design 28 on 2026-09-20. User-run regression passes:
261 CLI tests, 76 font tests and all dedicated tests, with zero failures or ignored
tests. Independent checks pass for 759 PDFs and 58 resource subsets. All 759 PDFs
remain byte-identical to the preceding stage; all 77 frozen source hashes match.
Overall design acceptance remains open.

## Decision

Use BookV2LineVariantBudget for both single-graph and multiple-graph replay. Add
budgeted entry points; the existing APIs delegate using a fresh owner. Replay
consumes accepted traversal, preparation and candidate charges without starting
a reshape feedback pass. The caller can share one owner across convergence,
context capture and replay. Successful views report only their invocation's work.

Recover partial candidate/frame/source-width work from the counted layout entry
point before propagating its error. Preserve the existing work from every earlier
graph in a set when a later graph fails. Also retain accepted set identity and
capacity traversal charges; an empty set or single-graph preflight rejection
before work remains uncharged. Preserve existing records and fingerprints.

The header catalog uses these APIs for its base replay and complete variant set.
Before entering the callback, retain replay failure work in its existing shared
budget. Once the callback starts, commit replay work once before downstream work;
a later failure neither refunds nor doubles it. Preserve the original stage and
error source. Returned view lifetimes remain tied to real reconstructed owners.

## Verification and remaining scope

Controlled TrueType and unchanged original Harano tests compare old/new single
and set output observations. They cover exact and one-short ceilings, failure
inside the second graph, repeated calls, consumer failure, empty/foreign seeds,
record rejection and one owner shared between seed creation and replay. Replay
must never consume additional reshape passes.

The full automatic table-header driver regression and 759-PDF independent/byte
comparisons pass. Independent verification covers 2,896 pages and rejects 5,795
mutations in the common PDF verifier. Shape internals, all initial frame
construction, source admission and other failed downstream phases still require
complete command accounting. Public Book /2, whole-book, performance, managed-host
and author/human acceptance remain open.
