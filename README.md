# typaxis

Typaxis は、再現可能な PDF を生成する Rust 製組版エンジンです。このリポジトリの文書はProfile 1.0の現行契約を記述します。契約・Schema・内部receiptの実装状況と、参照CLIからPDFまで到達できる機能範囲は同一ではありません。参照workspaceは意図的に限定されており、machine inputを含む現在の到達性は[Machine input PDF統合の不足機能・文書改善計画](docs/25-machine-input-pdf-improvements.md)を参照してください。

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
- [Testing strategy](docs/20-testing.md)
- [Implementation roadmap](docs/21-roadmap.md)
- [Cross-layer contract matrix](docs/22-contract-matrix.md)
- [Implementation checklist](docs/23-implementation-checklist.md)
- [Machine input PDF統合の不足機能・文書改善計画](docs/25-machine-input-pdf-improvements.md)

## 規範資料

- [`contracts/`](contracts/) — contract ID、phase ownership、横断 invariant
- [`schemas/`](schemas/) — canonical JSON/TOML Schema
- [ADR catalog](adr/README.md) — 採用済みの設計判断と適用範囲
- [`samples/`](samples/) — valid/invalid contract fixture

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
