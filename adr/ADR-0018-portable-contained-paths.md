# ADR-0018: Portable contained paths

## Status

Accepted in `typaxis.contract/1.0`.

## Decision

Package-internal source/resource paths use slash-separated relative `PortablePath` components. Absolute paths, dot components, backslashes, colons, NUL, and canonical resolution outside an admitted root are rejected there. Config roots instead use `ConfigResourceRoot = ProjectRoot | Relative(PortablePath)`; only ProjectRoot serializes as `.` and config roots are unique in UTF-8-byte order. Admitted roots are resolved config variants plus canonicalized/handle-backed explicit CLI host roots. CLI entry/output/config/resource-root paths are platform-native `HostPath` held by separate host contexts; they may be absolute but never enter EffectiveConfig, canonical artifacts, trace, or manifests. Manifests record only host-independent OutputSink for output, and BuildExecutionContext rejects aliasing file output/sidecar targets before writes. A declaration PortablePath is checked relative to every admitted root: zero regular-file candidates is not found, one is admitted, and more than one is an ambiguity error even when bytes match. Host root order never selects the first candidate. Admission records the resulting logical URI, bytes, and hash instead.

## Consequences

The rule is enforced in the Rust reference types, canonical schema or internal contract, validator, fixtures, and implementation checklist where applicable.
