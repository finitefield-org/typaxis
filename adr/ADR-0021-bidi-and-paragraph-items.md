# ADR-0021: Bidi and paragraph-item IR

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Bidi embedding levels remain attached to immutable shaped-run slices. After break selection each line applies UAX #9 L1 reset, final reshape, justification, then L2 visual reorder. Paragraph breaking consumes explicit Box/Glue/Penalty/Discretionary items with parsed TextSpan or full epoch-unique GeneratedProvenance (allocation-independent GeneratedBufferKey plus GeneratedTextSpan), and every Discretionary branch carries its drawing content rather than reconstructing spacing or content from glyph positions. Display construction remaps both internal text namespaces by TextBufferId/GeneratedBufferKey to dense DisplayTextSpan values before serialization/extraction.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
