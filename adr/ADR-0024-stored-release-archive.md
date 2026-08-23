# Stored reproducible release archive

- Status: Accepted
- Contract: `typaxis.contract/1.0`

## Decision

Release ZIP entries are uncompressed regular files with fixed timestamp, canonical mode, sorted path, and one versioned top-level directory whose name is independent of the checkout directory, avoiding zlib, ambient permission, and source-path variance.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
