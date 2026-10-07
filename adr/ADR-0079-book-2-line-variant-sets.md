# ADR-0079: Rebuild source-compatible line variants in one bounded owner

## Status

Accepted incrementally under design 28 on 2026-09-10. Makes multiple independent
line graphs available together without one recursive callback per variant.
Physical repeated-header selection, placement and PDF integration remain open.

## Decision

Rebuild a nonempty ordered set of converged seeds through separate preallocated
ownership layers: input contexts, authored shapes, prepared inlines, selected
lines and footnote-bound views. Hold every layer until one final callback returns.
No self-referential owner or recursion proportional to variant count is required.
Duplicate seed entries still produce distinct shape/line owners.

Before replay allocation, require exact common flow, policy, admitted resource,
vector-binding and optional native-computation owners, equal effective limits and
Japanese line-break mode. Identical content fingerprints do not authorize mixing
foreign flows or different native computation owners. Each seed retains its own
immutable page/body and width/origin inputs for the actual reconstruction.

Retain each seed's captured context charge separately from its prior base. Prepay
all listed captured-context bounds above the largest prior base (or preserve a
larger caller ledger), every observed replay graph bound, all paragraph input views,
five ownership-layer records per variant and one result record. Independent seed
prior charges can overlap; this is conservative accounting, not a minimal-memory
claim. Check arithmetic and the shared fragment ceiling before any replay vector
or shape is allocated.

Charge compatibility/preflight visits, context inputs, ownership-stage visits,
actual candidate/frame work and final comparisons to one work allowance. Preallocate
each outer vector once. Reconstruct from each seed's immutable inputs, require its
observed stable fingerprint and observed graph charge bound, and expose no views
unless every variant succeeds. An error never invokes the final callback.

The resulting set has an ordered geometry fingerprint incorporating every seed
fingerprint and position. All views carry the set's complete record charge;
individual view work preserves its direct replay cost, while set work includes
shared validation/stage overhead. Original source/native computations may be shared,
but prepared/shape/line/footnote identities remain separate and cross-owner checks
still fail. This set is not source-consumption or PDF authorization.

## Evidence and remaining work

Design 28 §14.213 and the progress ledger record controlled/Harano body/note sets
with narrow, wide and repeated narrow entries. Every graph is live simultaneously,
with independent table measurements and original header height differences. Exact
work/record limits, order fingerprints, empty-set/foreign-flow rejection and a
32-entry iterative reconstruction are checked. Native tests share one original
computation and reject a different computation owner despite equal fingerprints.

The set accounts for replayed shape/frame/line/footnote graphs. Initial seed capture
and subsequent body/table/page/PDF stages retain their existing accounting scopes;
complete repeated-header pipeline accounting and full-book performance remain open.
Per-occurrence header selection/reservation/placement, repeated source closure and
PDF assembly still need connection, along with other remaining design-28 work.
