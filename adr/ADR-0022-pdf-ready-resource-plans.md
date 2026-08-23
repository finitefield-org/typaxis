# PDF-ready resource plans

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

Font/image finalization produces validated subset mappings, descriptor metrics, color-space/bit-depth/encoding metadata, and cluster extraction plans before PDF object allocation.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
