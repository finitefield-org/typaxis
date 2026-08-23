# Stream ownership and page-tree validation

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

The serializer owns Length, Filter, and DecodeParms. Freeze validates reference closure plus Catalog/Pages/Page parent, kids, count, and cycle invariants.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
