# State-indexed pagination

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

Initial layout is state 0; pass i produces state i+1. Convergence, cycle start, and selected fallback state are expressed in the same nonnegative state-index space.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
