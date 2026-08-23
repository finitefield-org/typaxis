# ADR-0022: PDF-ready resource plans

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

An admitted resolver fixes font/image bytes, hashes, and pre-layout metadata before shaping from same-root-set sealed source receipts. A collector unions repeat use by logical ID, binds the supplied ledger fingerprint to the selected Display LayoutEpoch, and rejects one ID resolving to different admitted identity. The Display List remains PDF-independent. Profile 1.0 late finalization and downstream phases are PDF-specific: late finalization sorts font plans by `(font, admitted source SHA-256, FontInstanceId)` and image plans by `(image, admitted source SHA-256, ImageResourceId)`, rejects duplicate keys after use deduplication, and produces validated, PDF-ready but backend-identity-free `FrozenPdfResourcePlans` containing subset mappings, CID/CIDToGIDMap and extraction plans, closed descriptor metrics, color-space/bit-depth/encoding metadata, and exact typed indirect-object blueprints before PDF object allocation. A sealed subsetter receipt also binds the PostScript name re-extracted from the rewritten embedded program; finalization requires the exact deterministic FontInstanceId-derived subset name later used by all font dictionaries. It does not assign backend handles, PDF resource names, or object IDs.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
