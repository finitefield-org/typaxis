# typaxis

Typaxis は、再現可能な PDF を生成する Rust 製組版エンジンです。このリポジトリの文書はcurrent contract 1.3、凍結した1.0/1.1/1.2 input互換契約、ならびに非公開の1.4 design targetを記述します。契約・Schema・内部receiptの実装状況と、参照CLIからPDFまで到達できる機能範囲は同一ではありません。公開machine inputの範囲は[producer guide](docs/26-machine-input-cli.md)、M2以降の計画と状態は[Machine input PDF統合の不足機能・文書改善計画](docs/25-machine-input-pdf-improvements.md)を参照してください。

## 現行inputとmachine delivery status

`typaxis build INPUT`、`check`、`dump-ast`、`dump-layout`のINPUTはbounded **reference TSF**のままであり、content sniffingはしない。公開`build-package`と`check-package`は1.0/1.1/1.2/1.3 DocumentPackage JSONをsealed ingestionへ通し、`capabilities --format json`はcompiled seven-profile descriptorを出力する。supported reference TSFについては`dump-ast --format json -> build-package` round tripを提供する。

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| bounded reference TSF build | Yes, current 1.3 | Yes, reference subset | Yes | No |
| portable DocumentPackage validation/export | Yes, current 1.3 plus frozen 1.0/1.1/1.2 input | Yes: independent Schema registries/validator/export | Yes, through package commands | Yes |
| sealed DocumentPackage ingestion | Yes, ADR-0027 | Yes | Yes, macOS/Linux fixture gate | Yes, M1 host gate |
| `typaxis.machine-pdf/paragraph-1` | Yes, [capability contract](contracts/machine-pdf-capabilities.md) | Yes | Yes, macOS/Linux combined PDF/sidecars | Yes |
| `basic-document-1` / `table-1` / `footnote-1` | Yes, ADR-0028/0029/0030 | Yes | Yes, combined PDF/sidecars | Yes, profile gates |
| contract 1.3 generated artifacts | Yes | Yes | Yes | Yes |
| `header-footer-1` / `columns-1` / `float-1` | Yes, ADR-0031 | Yes: selected-state, Display/PDF, and artifact closure | Yes, combined PDF/sidecars | Yes, MI3-12 gate |
| contract 1.4 / `production-book-1` target | Yes through ADR-0037: base/media, native math/safe-vector, producer-composed math vector, metadata/language/outline, tagged PDF/PDF/UA-1 validation, and separate baseline-JPEG/CFF1 components | Existing private slices through MI4-09 plus MI4-V01 corpus; producer-vector product work remains MI4-V03〜V18, V19 is evidence, and JPEG/CFF remain MI4-11/12 | No; public CLI remains 1.3/seven-profile | No; MI4-V19 then MI4-13 gate |

`Contract-defined`はRust crate、public command、fixture E2E、release supportの存在を意味しない。M1は、明示的に管理するLinux・macOS hostで同一revision/source/artifactのactual evidenceを生成・集約して`Release-supported`となった。GitHub Actionsは使用していない。

## 設計文書

### 概要とデータモデル

- [目的・非目的・品質目標](docs/00-goals-and-scope.md)
- [全体アーキテクチャ](docs/01-architecture.md)
- [Rust workspace 境界](docs/02-workspace-boundaries.md)
- [Source、TextStore、Parser 契約](docs/03-source-text-and-parser.md)
- [Document、Style、Resource model](docs/04-document-style-resource-model.md)

### 組版パイプライン

- [Text pipeline](docs/05-text-pipeline.md)
- [日本語改行・禁則・字間](docs/06-japanese-line-breaking.md)
- [Paragraph layout](docs/07-paragraph-layout.md)
- [Re-entrant fragmentation](docs/08-reentrant-fragmentation.md)
- [Pagination と収束](docs/09-pagination-and-convergence.md)
- [Table、footnote、column、float](docs/10-tables-footnotes-floats.md)

### 出力と信頼性

- [Display List 契約](docs/11-display-list.md)
- [Font、subset、text extraction](docs/12-fonts-subsetting-extraction.md)
- [Deterministic resource finalization](docs/13-resource-finalization.md)
- [PDF object model](docs/14-pdf-object-model.md)
- [PDF text、link、coordinate mapping](docs/15-pdf-text-links-and-coordinates.md)
- [Determinism、spooling、build manifest](docs/16-determinism-spooling-manifest.md)
- [Diagnostics と trace](docs/17-diagnostics-observability.md)
- [Security と resource limits](docs/18-security-and-limits.md)
- [Units、rounding、geometry](docs/24-units-rounding-and-geometry.md)

### 利用・実装・検証

- [CLI](docs/19-cli.md)
- [Machine input CLI producer guide](docs/26-machine-input-cli.md)
- [Testing strategy](docs/20-testing.md)
- [Implementation roadmap](docs/21-roadmap.md)
- [Cross-layer contract matrix](docs/22-contract-matrix.md)
- [Implementation checklist](docs/23-implementation-checklist.md)
- [Machine input PDF統合の不足機能・文書改善計画](docs/25-machine-input-pdf-improvements.md)
- [VMB向け組版済み数式ベクター配置の実装設計](docs/27-vmb-precomposed-math-vector.md)

## 規範資料

- [`contracts/`](contracts/) — contract ID、phase ownership、横断 invariant
- [`schemas/`](schemas/) — canonical JSON/TOML Schema
- [ADR catalog](adr/README.md) — 採用済みの設計判断と適用範囲
- [`samples/`](samples/) — valid/invalid contract fixture
- [Machine package fixture README](samples/machine-package/README.md) — runnable M1 packageとhost evidence gate

## 実装

参照用 Rust workspace の構成と実行方法は [workspace/README.md](workspace/README.md) を参照してください。

## Release と再現性検証

Release ZIP は指定した Git tree だけから生成し、checkout 名、working tree、Cargo の
`target` 出力を含めません。生成と canonical metadata/payload の再検証は次で行います。

```console
python3 tools/release.py --repository . --revision HEAD --verify dist/typaxis-0.1.0.zip
```

異なる名前の二つの独立 checkout で blank PDF と release ZIP をそれぞれ生成し、bytes を
exact 比較する検証（ambient `TYPAXIS_*` config は除外）は次です。

```console
python3 tools/verify_reproducibility.py --repository . --revision HEAD
python3 -m unittest discover -s tools -p 'test_*.py' -v
```

MuPDFとPopplerを使う独立renderer/extractor gateは、比較する二つのPDFに対して次のように実行します。

```console
python3 tools/verify_pdf_differential.py --pdf first.pdf --pdf second.pdf --expected-pages 1 --expected-text 'expected'
```

公開machine profileのclean build、二回実行、異名source snapshot再現性、Schema、MuPDF/Poppler、host evidenceをまとめたgateは次で実行する。

```console
python3 tools/verify_machine_profile.py \
  --repository . \
  --fixture samples/machine-package/profiles/paragraph-1/combined/expected.json \
  --runs 2 --require-external-tools
```

Linux/macOS evidenceの集約方法は[fixture README](samples/machine-package/README.md#release-and-host-evidence)を参照する。片方のevidence欠落、failed/noncanonical/stale revision、異なるsource/fixture/artifactはrelease gateのfailureである。
