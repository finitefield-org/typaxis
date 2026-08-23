# ADR-0019: State-indexed pagination

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Initial layout seed is state 0 and is never selectable; pass i produces materialized state i+1. The seed and materialized records use domain-separated fingerprint encodings. A cycle can repeat only a prior materialized state, so its start is in 1 through pass_count; state 0 is neither a cycle target nor a fallback candidate. `PaginationFingerprintEncoder` first converts each state into arrays with declared unique sort keys for pages, frames, fragments, footnotes, floats, columns, and generated text, excluding allocation IDs, and only then applies JCS/SHA-256. Cycle/max-pass fallback candidates are materialized states 1 through pass_count and are selected by the canonical `lowest_cost_then_earliest` score `(hard_violation_count,total_cost,page_count,state_index)`.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
