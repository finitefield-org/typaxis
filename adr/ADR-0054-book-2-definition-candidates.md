# ADR-0054: Bounded source candidates for mixed footnote definitions

## Status

Accepted incrementally under design 28 on 2026-09-10. This supplies automatic
source candidate enumeration for common footnote integration. Multi-definition
reservation, dependency backtracking, physical page placement and full-book/public/
human acceptance remain required.

## Decision

Enumerate ordinary item boundaries and each legal final table cut using the actual
mixed definition source state. Continue an ordinary/table prefix only after the
preceding table completes without an authored forced break. A serial forced break
is consumed by its preceding fitting prefix or as a leading zero-height candidate;
it is not deferred solely to create an additional blank region.

Use the common body boundary cost calculation with the declared available note
capacity and the original paragraph/heading context. Order equal-cost definition
candidates by furthest source progress, including the independent root table
ordinal. This lets adjacent and trailing zero-height tables finish in the current
candidate when they fit. A zero-capacity candidate can consume only zero-height
source events, and its cost calculation must not divide by zero. The body owner's
existing tie policy is unchanged.

Enumerate the complete bounded candidate set before issuing a best choice. Retain
all feasible source/demand branches in cost order so a reservation owner can later
backtrack through actual alternatives. Enforce the configured lookback, record and
work bounds while enumerating, copying requests, ranking, re-evaluating and retaining
choices. An early feasible choice does not excuse exhaustion of the remaining
search. Failed or discarded candidates do not refund budget or modify the incoming
demand state. Each choice and the set itself bind the exact search/state identity.

The earlier-table-capacity query is shared by body and definition contexts. It
uses the original measured global source leaves, including nested caption/cell
geometry, rowspan continuation bands and forced source breaks. Selection still
translates retained leaves to their original definition-local indexes before
requiring references. Repeated headers do not add a semantic reference occurrence.

## Evidence and scope

Design 28 §14.188 and the progress ledger record automatic enumeration across
ordinary content, keeps, empty tables, forced breaks, nested captions/headers,
rowspans and deep nesting. They also cover source-caused early/late/repeated-header
references, source-once traversal, equal-cost empty-table progress, zero capacity,
a lookback refusal despite an early feasible boundary and exact/one-short budgets.
The original common page entry point retains its definition-table guard. This
source candidate set is not a physical note reservation or PDF receipt.
