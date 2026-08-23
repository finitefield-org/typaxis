# Typaxis contract version

Canonical wire identifier:

```text
typaxis.contract/1.0
```

Every canonical JSON root and `typaxis.toml` must carry this exact value. JSON Schema IDs use `/1.0/` under the Typaxis schema namespace. The design-package release is `1.0.0`; the reference Rust workspace uses crate version `0.1.0`. Release and crate versions do not substitute for the wire-contract identifier.

A change to field meaning, identifier space, coordinate unit, convergence semantics, extraction ownership, path policy, or PDF resource ownership requires a new Typaxis wire-contract major/minor value.
