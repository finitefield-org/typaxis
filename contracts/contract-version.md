# Typaxis contract version

Canonical wire identifier:

```text
typaxis.contract/1.0
```

Every canonical JSON root and `typaxis.toml` must carry this exact value. JSON Schema IDs use `/1.0/` under the Typaxis schema namespace. The design-package release is `1.0.0`; the reference Rust workspace uses crate version `0.1.0`. Release and crate versions do not substitute for the wire-contract identifier.

The contract remains a draft until the repository has a matching release tag for the design-package release. Before that tag, consistency fixes update the same `typaxis.contract/1.0` draft and do not increment the contract ID. After publication, an incompatible change to field meaning, identifier space, coordinate unit, convergence semantics, extraction ownership, path policy, or PDF resource ownership requires a new major value; a backward-compatible additive wire change requires a new minor value. Editorial changes that do not alter observable contract meaning do not increment the value.
