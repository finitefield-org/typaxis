# Bidi and paragraph-item IR

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

Bidi embedding levels remain attached to runs, and paragraph breaking consumes explicit Box/Glue/Penalty/Discretionary items rather than reconstructing spacing from glyph positions.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
