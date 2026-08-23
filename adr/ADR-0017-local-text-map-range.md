# ADR-0017: Local text-map ranges

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Text-map segments use an owning-buffer-local `Utf8ByteRange`, not a cross-buffer `TextSpan`. This makes a segment incapable of naming another text buffer. An identity segment additionally requires equal byte lengths and byte-for-byte equality between its text and source ranges.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
