# Typaxis contract version

Canonical current-output wire identifier:

```text
typaxis.contract/1.1
```

Current canonical JSON encoders and normalized `EffectiveConfig` values emit this exact value. Current JSON Schema IDs use `/1.1/` under the Typaxis schema namespace. The previous seven-schema 1.0 registry is frozen byte-for-byte under `schemas/1.0/`; it is a separate registry and is never populated with a 1.1 shape. The design-package release is `1.0.0`; the reference Rust workspace uses crate version `0.1.0`. Release and crate versions do not substitute for the wire-contract identifier.

DocumentPackage input parsing recognizes the closed set `typaxis.contract/1.0` and `typaxis.contract/1.1`. A raw 1.0 configuration is likewise accepted as migration input, receives the 1.1 defaults for fields that did not exist in 1.0, and is normalized to a 1.1 `EffectiveConfig` before hashing. Compatibility input never changes the producer rule: diagnostics, manifests, traces, display lists, normalized configs, capabilities, and `dump-ast` output use 1.1 only.

The contract remains a draft until the repository has a matching release tag for the design-package release. An incompatible change to field meaning, identifier space, coordinate unit, convergence semantics, extraction ownership, path policy, or PDF resource ownership requires a new major value; a backward-compatible additive wire change requires a new minor value. Editorial changes that do not alter observable contract meaning do not increment the value. In particular, a new shape must never be published under the frozen `typaxis.contract/1.0` identifier.

## Accepted non-current 1.2 target

[ADR-0028](../adr/ADR-0028-basic-document-profile.md) reserves `typaxis.contract/1.2` and versioned DocumentPackage Schema `$id` `https://schemas.typaxis.invalid/1.2/document-package.schema.json`. The additive shape extends the closed style-property enum with `space_before`, `space_after`, `start_indent`, `end_indent`, `text_align`, `width`, `keep_with_next`, and `keep_caption`; their exact tagged values and semantics are fixed by that ADR.

This reservation does not change the canonical current-output identifier above. Before MI2-08, public decode rejects 1.2, public capabilities omit `basic-document-1`, `schemas/*.schema.json` remains the 1.1 registry, and only crate-private M2 staging tests may use versioned Schemas under `schemas/1.2/`. No 1.2 staging artifact is a release-support claim.

MI2-08 is one atomic migration. It freezes the former current 1.1 registry under `schemas/1.1/`, completes the independent 1.2 registry, then switches `typaxis_core::CONTRACT`, current Schema aliases, encoders, decoder registry, generated config/package/trace/diagnostics/manifest/capability artifacts, fixtures, and `dump-ast` output together. After that switch, DocumentPackage input recognizes the closed set 1.0/1.1/1.2; `basic-document-1` requires raw 1.2, while `paragraph-1` continues to accept only its frozen semantic subset and remains the default profile. Unknown contracts never upgrade to the newest version.
