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
| bounded reference TSF pipeline | Yes, current 1.3 | Yes, reference subset | Yes | No |
| DocumentPackage portable Schema and `dump-ast` export | Yes, current 1.3 plus frozen 1.0/1.1/1.2 input | Yes: independent registries and shared export | Yes, package round trip | Yes, M1 host gate |
| MI0-01 macOS baseline | Yes | Completed | blank reference smoke only | Not a machine release gate |
| MI0-02 machine ingestion architecture | Yes, ADR-0027 | Yes: owner graph and sealed receipts | Yes, through M1 commands | Not a standalone release gate |
| M1 `typaxis.machine-pdf/paragraph-1` | Yes, closed capability contract | Yes | Yes, macOS/Linux combined fixture | Yes |
| contract 1.3 generated artifacts | Yes | Yes, current output with frozen 1.0/1.1/1.2 input | Yes | Yes |
| M2 `basic-document-1` and M3 `table-1` / `footnote-1` | Yes, ADR-0028/0029/0030 | Yes | Yes, combined fixtures | Yes, profile gates |
| M3 `header-footer-1` / `columns-1` / `float-1` | Yes, ADR-0031 on current contract 1.3 | Yes: selected-state and artifact closure | Yes, combined fixtures | Yes, MI3-12 gate |
| M4 contract 1.4 / `production-book-1` assembled target | Yes through ADR-0035 for base/media, math/safe-vector, metadata/language/outline, and tagged PDF/PDF/UA-1 validation; JPEG/OTF-CFF decisions pending | Partial private MI4-02/04/05/07/09 slices, including tagged structure and writer-independent PDF validation | No; current public surface remains 1.3 | No, MI4-13 gate |
| remaining M4 JPEG/OTF-CFF decisions and M5 gates | No until their assigned ADRs | No | No | No |

現行`build` INPUTはreference TSFで、DocumentPackage JSONは別の公開`build-package`/`check-package`へ入力する。`capabilities --format json`を含むpublic CLI E2E、producer guide、再現性・external PDF gate、同一revision/source/artifactのmacOS/Linux actual evidence集約は完了した。GitHub Actionsは使用していない。

## Machine-input rollout order

1. MI0-01: macOS build baselineとactual-host evidence。
2. MI0-02: ADR、owner graph、immutable profile、status axes（本milestone）。
3. MI1-01〜MI1-09: crate boundary、strict decode、host/source admission、syntax trust、diagnostics。
4. MI1-10〜MI1-13: profile/preflight、layout boundary、manifest progress、terminal publication。
5. MI1-14: contract 1.0 registryをfreezeし、generated artifactsを1.1へatomic switch。
6. MI1-15〜MI1-16: non-public orchestrationとinternal E2E fixture closure。
7. MI1-17: public command registration、producer docs、documented-host/reproducibility gate、actual two-host evidence aggregation。
8. MI3-08: contract 1.3とadvanced-pagination profile splitを採択するが、current 1.2/public descriptorsは変更しない。
9. MI3-09〜MI3-11: header/footer、columns、floatをcrate-private stagingとして実装する。
10. MI3-12: full 1.3 Schema/encoder/decoder/artifact migration、3 profile、`m3-all.json`を一つのpublication gateで公開する。
11. MI4-01: non-current contract 1.4、closed semantic container、required declared media、`production-book-1`とatomic migrationを採択する。
12. MI4-02〜MI4-12: M4 ADR群が採択したsliceをcrate-private 1.4 stagingとして実装し、current 1.3/public bytesを維持する。
13. MI4-13: complete 1.4 registry、resource-attested export、artifact version dispatch、production fixture/profileを一つのpublication gateで公開する。

ADR-0032はM4のbase contract/container/media ownership、ADR-0033はmath/safe-vector/alternative、ADR-0034はmetadata/language/outline、ADR-0035はsource-boundでlayout-contract-ownedのtagged PDF registry、artifact/MCID closure、PDF/UA-1 validation evidenceをcontract-definedへ昇格した。MI4-02/04/05/07/09はこれらのsliceをprivate 1.4 stagingへ実装済みである。JPEG/OTF-CFFの具体policyは後続decision gateに属する。ADR-0031で採択したadvanced paginationはMI3-09〜MI3-11のprivate implementationをMI3-12で一括公開した。既存7 profileの意味と`paragraph-1` defaultは変更していない。

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
