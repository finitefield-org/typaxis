# Implementation roadmap

このroadmapは実装順とdelivery statusを分ける。契約・Schema・reference type/testが存在しても、公開CLIからPDFまでend-to-endで到達できるとは限らない。machine input milestoneの依存関係、受入条件、Completed記録は[docs/25 task plan](25-machine-input-pdf-improvements-todo.md)を正とする。

## Status axes

| Axis | Meaning |
| --- | --- |
| Contract-defined | ADR/contract/Schemaが意味を固定した。Rust implementationの存在は含まない |
| Implemented | 対象ownerのcode/testが存在する。`Partial`は公開可能という意味ではない |
| Public CLI E2E | documented public binary commandでfixtureからobservable artifactまで検証済み |
| Release-supported | documented host/reproducibility/profile evidenceをrelease gateで閉じた |

## Current and target delivery matrix

| Capability | Contract-defined | Implemented | Public CLI E2E | Release-supported |
| --- | --- | --- | --- | --- |
| bounded reference TSF pipeline | Yes, current 1.1 | Yes, reference subset | Yes | No |
| DocumentPackage portable Schema and `dump-ast` export | Yes, current 1.1 plus frozen 1.0 input | Yes: dual validator/export | Yes, package round trip | No |
| MI0-01 macOS baseline | Yes | Completed | blank reference smoke only | Not a machine release gate |
| MI0-02 machine ingestion architecture | Yes, ADR-0027 | Yes: owner graph and sealed receipts | Yes, through M1 commands | No |
| M1 `typaxis.machine-pdf/paragraph-1` | Yes, closed capability contract | Yes | Yes, Linux combined fixture | No: two-host aggregate pending |
| contract 1.1 generated artifacts | Yes | Yes, current output | Yes | No: two-host aggregate pending |
| M2-M5 rich machine profiles | No: decision-gate ADR pending | No | No | No |

現行`build` INPUTはreference TSFで、DocumentPackage JSONは別の公開`build-package`/`check-package`へ入力する。`capabilities --format json`を含むpublic CLI E2EはLinuxで成功済みであり、producer guideと再現性・external PDF・host evidence gateも実装済みである。release列は同一revision/source/artifactのmacOS/Linux actual evidenceがCIで集約されるまで`No`のままとする。

## Machine-input rollout order

1. MI0-01: macOS build baselineとactual-host evidence。
2. MI0-02: ADR、owner graph、immutable profile、status axes（本milestone）。
3. MI1-01〜MI1-09: crate boundary、strict decode、host/source admission、syntax trust、diagnostics。
4. MI1-10〜MI1-13: profile/preflight、layout boundary、manifest progress、terminal publication。
5. MI1-14: contract 1.0 registryをfreezeし、generated artifactsを1.1へatomic switch。
6. MI1-15〜MI1-16: non-public orchestrationとinternal E2E fixture closure。
7. MI1-17: public command registration、producer docs、documented-host/reproducibility gate、actual two-host evidence aggregation。

M2以降のprofile ID、page-break blank-page policy、table split policy、math/vector/book publicationは各decision-gate ADRがAcceptedになるまでcontract-definedへ昇格しない。M1の`paragraph-1`を拡張して代用しない。

## Existing reference capability history (not completion status)

以下はreference workspaceを構成してきた技術targetの分類で、上表のdelivery statusやdocs/25のmilestone IDではない。

- M0 contract: scoped core newtypes、separate SourceCatalog/TextStore ownership、`ValidatedParsedPackage`/`ParseOutcome`/`AdvisoryDiagnostic`、SafeUri、structured Fragmenter continuation、Display/PDF model、validator。
- M1 minimal PDF: path/JPEG design、Type0/CIDFontType2、CIDToGIDMap、ToUnicode、absolute Japanese run、xref。JPEG/figure paintは現行runtime未完成。
- M2 text: admitted resource resolution、grapheme/bidi/itemization/fallback/shaping、line-level UAX #9 reorder、cluster extraction round-trip。
- M3 paragraph: UAX #14、Japanese pair table、greedy/optimal、justification。
- M4 flow/pagination: LayoutPassCoordinator feedback、paragraph/heading/list/image、page masters、keep/widow/orphan、scored fallback、trace convergence。
- M5 table/footnote/reference: basic table、bounded footnote、TOC/page reference。
- M6 hardening: deterministic spool/release、limits、fuzzing、renderer/extractor matrix、accessibility investigation。

各delivery milestoneは対応するobservable acceptance testでだけ完了判定する。
