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
| atomic PDF/sidecar publication | Available for current reference CLI | Unix committerとlocked E2Eが個別atomic/no-replace/replace/partial-publication規則を検証 |
| contained machine PACKAGE/source open | Not part of the MI0 baseline | current machine host admission and `build-package` are verified by the separate MI1 gate below |
| declared font/imageのcontained resource open | Unsupported in MI0 | exit 3、stable `UnsupportedContainedOpen`、requested PDF/manifestは作成・置換しない |

上表はMI0-01時点のhistorical baselineである。current worktreeのmacOS/Linux contained package/source/font pathはMI1 public gateで検証済みであり、current-source actual evidenceはlocal aggregation commandで閉じた。GitHub Actionsは使用していない。AndroidはM1 release-supported hostではない。

## Machine input status

現行binaryの`build` INPUTはreference TSFであり、DocumentPackage JSONをsniffしない。DocumentPackage 1.0/1.1は公開`build-package`/`check-package`へ入力し、`capabilities --format json`はcompiled descriptorをcanonical JSONで返す。supported reference TSFでは`dump-ast --format json -> build-package` round tripが成立する。詳細は[producer guide](../docs/26-machine-input-cli.md)と[runnable fixture README](../samples/machine-package/README.md)を参照する。

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| reference TSF pipeline | Yes, current 1.1 | Yes, bounded reference subset | Yes | No |
| DocumentPackage portable Schema/export | Yes, current 1.1 plus frozen 1.0 input | Yes | Yes, package round trip | Yes, M1 host gate |
| sealed package/source admission | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, closed capability contract | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| contract 1.1 generated artifacts | Yes | Yes, current output | Yes | Yes, M1 host gate |

Portable validation、Rust owners、public CLI E2E、release supportは別のstatus軸である。M1 release supportは同一revision/source/artifactのcurrent Linux/macOS actual evidenceを集約して完了した。

repository rootからpublic current-host gateを実行するにはMuPDFとPopplerを用意し、次を使う。

```text
python3 tools/verify_machine_profile.py \
  --repository . \
  --fixture samples/machine-package/profiles/paragraph-1/combined/expected.json \
  --runs 2 --require-external-tools
```

Resource trust is split deliberately: `typaxis-resource-admission` owns sealed root/source receipts, bounded bytes/hash/metadata, the immutable ledger, and canonical font-instance identity. `typaxis-layout-contract` owns `LayoutEpoch`, admitted style resolution, and the package/style/ledger/instance-bound font-selection receipt shared by layout and shaping. `typaxis-resources` begins only at selected Display usage union and PDF-profile subset/image finalization. `typaxis-pdf` alone preflights all typed indirect-object roles and issues the publication `FrozenPdfGraph`; its public low-level builder remains explicitly untrusted.

Parser trust is source-driven. The sealed `typaxis-syntax::ReferenceParser` implements only the small record grammar needed to compile-check downstream boundaries (`paragraph`, `anchor:<id>`, and `text:<utf8>`); it derives the AST, IDs, spans, maps, and default master internally. No Cargo feature exposes an arbitrary `ParsedPackage -> ValidatedParsedPackage` promotion path.
