# Rust reference workspace

This reference workspace makes type boundaries compile-checkable and checks forbidden dependency edges in `typaxis-testkit`. It is not a completed Typaxis typesetting engine; its CLI exercises the deliberately bounded reference parser and layout domain. Its small set of host-I/O and shaping dependencies is exact-pinned in `Cargo.lock` for reproducibility. From the repository root, use Rust 1.75 or later and run:

```text
cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path workspace/Cargo.toml --workspace --locked
cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo fmt --manifest-path workspace/Cargo.toml --all -- --check
```

Resource trust is split deliberately: `typaxis-resource-admission` owns sealed root/source receipts, bounded bytes/hash/metadata, the immutable ledger, and canonical font-instance identity. `typaxis-layout-contract` owns `LayoutEpoch`, admitted style resolution, and the package/style/ledger/instance-bound font-selection receipt shared by layout and shaping. `typaxis-resources` begins only at selected Display usage union and PDF-profile subset/image finalization. `typaxis-pdf` alone preflights all typed indirect-object roles and issues the publication `FrozenPdfGraph`; its public low-level builder remains explicitly untrusted.

Parser trust is source-driven. The sealed `typaxis-syntax::ReferenceParser` implements only the small record grammar needed to compile-check downstream boundaries (`paragraph`, `anchor:<id>`, and `text:<utf8>`); it derives the AST, IDs, spans, maps, and default master internally. No Cargo feature exposes an arbitrary `ParsedPackage -> ValidatedParsedPackage` promotion path.
