# Rust reference workspace

This reference workspace makes type boundaries compile-checkable and checks forbidden dependency edges in `typaxis-testkit`. It is not a completed Typaxis typesetting engine; its CLI exercises the deliberately bounded reference parser and layout domain. Its small set of host-I/O and shaping dependencies is exact-pinned in `Cargo.lock` for reproducibility. From the repository root, use Rust 1.75 or later and run:

```text
cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked
cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings
cargo fmt --manifest-path workspace/Cargo.toml --all -- --check
```

CLI binaryのbuildability、version、最小blank PDFを対象source revisionで確認するgateは、repository rootから空のtarget directoryを使って次を実行する。

```text
MI0_TARGET="$(mktemp -d /tmp/typaxis-mi0-target.XXXXXX)"
CARGO_TARGET_DIR="$MI0_TARGET" cargo build --manifest-path workspace/Cargo.toml --package typaxis-cli --locked
"$MI0_TARGET/debug/typaxis" --version
"$MI0_TARGET/debug/typaxis" build samples/minimal/empty.tsf -o "$MI0_TARGET/typaxis-mi0-empty.pdf"
```

`workspace/target/debug/typaxis`が既に存在していても、現在のsource revisionからbuildされたとは限らない。integrationや再現性検証では上記のように空のtarget directoryからbuildするか、明示的にadmitしたrelease binaryのversionとdigestを記録する。

MI0-01のactual-host gateはimplementation commit `edd8ec9f57a2a58de6f6c23af94b1982fb4da9d1`で完了した。macOS 26.5.2 arm64、rustc/cargo 1.97.1でlocked build/check/workspace all-targets test/clippyとfmt checkが成功した。空のtarget directoryから作ったdebug binaryは`typaxis 0.1.0`、SHA-256 `6c2364768483afc97ed8fd2502a54ca47ea61d0efb640872ad576f2d2a3a9ade`だった。そのbinaryによるblank smokeは512-byteのPDF 1.7（1 page）、SHA-256 `01bdd2e1b730cab33456b08582ec237ef155ad90f33ca5d1731a9132adb48e8e`を生成した。

| MI0 capability | macOS status | Observable contract |
| --- | --- | --- |
| workspace compile/test/lint | Available | locked build/check/all-targets test/clippyがexit 0 |
| resourceなしreference TSFからblank PDF | Available | `typaxis 0.1.0`が1-page PDFを生成 |
| declared font/imageのcontained resource open | Unsupported in MI0 | exit 3、stable `UnsupportedContainedOpen`、requested PDF/manifestは作成・置換しない |

Linux/Androidのcontained resource pathは実装domainに残るが、MI0-01ではruntimeを再検証していない。documented Linux gateはMI1-17で閉じる。macOS resource admissionを成功扱いせず、MI0では上表のfail-closed contractだけを保証する。

Resource trust is split deliberately: `typaxis-resource-admission` owns sealed root/source receipts, bounded bytes/hash/metadata, the immutable ledger, and canonical font-instance identity. `typaxis-layout-contract` owns `LayoutEpoch`, admitted style resolution, and the package/style/ledger/instance-bound font-selection receipt shared by layout and shaping. `typaxis-resources` begins only at selected Display usage union and PDF-profile subset/image finalization. `typaxis-pdf` alone preflights all typed indirect-object roles and issues the publication `FrozenPdfGraph`; its public low-level builder remains explicitly untrusted.

Parser trust is source-driven. The sealed `typaxis-syntax::ReferenceParser` implements only the small record grammar needed to compile-check downstream boundaries (`paragraph`, `anchor:<id>`, and `text:<utf8>`); it derives the AST, IDs, spans, maps, and default master internally. No Cargo feature exposes an arbitrary `ParsedPackage -> ValidatedParsedPackage` promotion path.
