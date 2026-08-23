# Display destinations and paint

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

The Display List carries explicit glyph paint and a unique named-destination table. Internal annotations must resolve before PDF resource finalization.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
