# ADR-0020: Display destinations and paint

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

The Display List carries explicit glyph paint and a unique named-destination table. The table is derived exactly from the selected pagination receipt's package-registered, page/frame-bound anchor placements; callers cannot add, omit, or relocate destinations. Internal annotations must resolve before PDF resource finalization. URI targets are validated `SafeUri` values, and annotation/destination coordinates are converted outside the content CTM using the page height. The PDF backend materializes the complete table as Catalog Names/Dests, creates one indirect Link annotation per Display occurrence, and references each exactly once from its owning page Annots array; unsupported raw actions cannot enter the frozen graph.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
