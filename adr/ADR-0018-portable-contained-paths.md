# Portable contained paths

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

All local source/resource/build paths use slash-separated relative components. Absolute paths, dot components, backslashes, colons, NUL, and canonical resolution outside an admitted root are rejected.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
