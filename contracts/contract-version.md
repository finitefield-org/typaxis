# Typaxis contract version

Canonical current-output wire identifier:

```text
typaxis.contract/1.1
```

Current canonical JSON encoders and normalized `EffectiveConfig` values emit this exact value. Current JSON Schema IDs use `/1.1/` under the Typaxis schema namespace. The previous seven-schema 1.0 registry is frozen byte-for-byte under `schemas/1.0/`; it is a separate registry and is never populated with a 1.1 shape. The design-package release is `1.0.0`; the reference Rust workspace uses crate version `0.1.0`. Release and crate versions do not substitute for the wire-contract identifier.

DocumentPackage input parsing recognizes the closed set `typaxis.contract/1.0` and `typaxis.contract/1.1`. A raw 1.0 configuration is likewise accepted as migration input, receives the 1.1 defaults for fields that did not exist in 1.0, and is normalized to a 1.1 `EffectiveConfig` before hashing. Compatibility input never changes the producer rule: diagnostics, manifests, traces, display lists, normalized configs, capabilities, and `dump-ast` output use 1.1 only.

The contract remains a draft until the repository has a matching release tag for the design-package release. An incompatible change to field meaning, identifier space, coordinate unit, convergence semantics, extraction ownership, path policy, or PDF resource ownership requires a new major value; a backward-compatible additive wire change requires a new minor value. Editorial changes that do not alter observable contract meaning do not increment the value. In particular, a new shape must never be published under the frozen `typaxis.contract/1.0` identifier.
