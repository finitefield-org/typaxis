# ADR-0023: Stream ownership and page-tree validation

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

The serializer owns Length, Filter, and DecodeParms materialization from frozen payload/filter policy. Duplicate object insertion leaves the builder unchanged, returns an error, and stops the build. Freeze validates reference closure plus Catalog/Pages/Page parent, kids, count, cycle, Catalog Names/Dests, and page Annots invariants. Direct-value and page-tree walks are iterative and reject depth above the inclusive profile maximum 64. The indirect-body root PdfValue, stream dictionary, and root Pages node are depth 1; every direct child or page-tree child increments depth. Recursive PdfValue payloads are drained iteratively on drop.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
