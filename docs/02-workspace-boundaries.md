# Rust workspace境界

## Current 1.0 reference boundary

この節は現行workspaceの実装境界である。後段のM1 target crateはADR-0027で採択済みだが未作成であり、ここに記載しただけでmachine inputがimplemented、CLI E2E、release-supportedになったとは扱わない。

`typaxis-core`を最下層とし、document/styleは互いに依存しない。syntaxだけがsources/text/document/style/resourcesを束ねる。layoutはPDFを知らず、PDFはDocument ASTを知らない。

`typaxis-resource-admission`は`core + document + font`だけに依存する下位trust crateであり、configured root-set token、contained source receipt、resolver-session-bound bounded bytes/hash、bytes-derived metadata、complete ledger、canonical font-instance tableを所有する。pending bytes/metadata receiptは発行元resolverのopaque session identityを保持し、別limitsで作ったresolverへ移してbudgetを迂回できない。`typaxis-layout-contract`は`core + resource-admission + style + syntax`だけに依存し、`LayoutEpoch`、admission-bound resolved text style、package/style/ledger/canonical instance tableを同時照合した`ShapeFontSelectionReceipt`を所有する。`layout`はこれらをre-exportし、`shaping`はraw font instanceではなくreceiptだけを受ける。`typaxis-resources`はDisplay usage union、subset/image encoder receipt、PDF-ready final planだけを所有し、host path admissionを再実装しない。

禁止edge:

- `typaxis-core -> workspace crate`
- `typaxis-document <-> typaxis-style`
- `typaxis-layout-contract -> core|resource-admission|style|syntax以外のworkspace crate`
- `typaxis-layout -> typaxis-display-list|typaxis-pdf`
- `typaxis-pagination -> typaxis-pdf`
- `typaxis-pdf -> typaxis-document|typaxis-style|typaxis-layout|typaxis-pagination`
- `typaxis-resource-admission -> typaxis-style|typaxis-syntax|typaxis-layout|typaxis-shaping|typaxis-display-list|typaxis-resources|typaxis-pdf`

newtypeはcross-phaseで受け渡すsemantic ID (`SourceId`、`TextBufferId`、`GeneratedTextBufferId`、`DisplayTextBufferId`、`NodeId`、`FontInstanceId`、`ImageResourceId`、`LayoutStateIndex`、`PageName`)、混同可能なidentifier (`OriginalGid`、`SubsetGid`、`Cid`)、単位付き値 (`Length`、source/text byte offset)、検証済み値 (`PortablePath`、`ConfigResourceRoot`、`SafeUri`)、host非依存の出力分類 (`OutputSink`) に要求する。同じunderlying整数/文字列でもparsed/generated/Display text IDやPageName/StyleId/MasterIdを暗黙変換しない。`ConfigResourceRoot::ProjectRoot`だけがwire `"."`を表し、`PortablePath`へ変換しない。crate内だけのloop index、collection index、countまで一律にnewtype化せず、Display wire境界のdense remapのように混同防止が必要なindexだけを型で区別する。source/text offsetに裸の`usize`を使用せず、cross-phase fingerprintは対象state型とalgorithm IDを持つ専用型にする。

## Accepted M1 machine-input target

[ADR-0027](../adr/ADR-0027-machine-document-package-ingestion.md)は次のowner graphをM1 targetとして固定する。これはtarget designであり、crateの存在やpublic commandを表すimplementation inventoryではない。

| crate | Sole responsibility | Forbidden responsibility |
| --- | --- | --- |
| `typaxis-host-admission` | directory handle/root set、contained open、same-handle snapshot、stable bounded read、host read/write identity ledger | package/resource logical ID、JSON/domain decode、canonical artifact projection |
| `typaxis-document-package` | caller-constructible untrusted `WireDocumentPackage`、strict JSON preflight/decode、decoder-issued `DecodedDocumentPackage`、JSON Pointer index、JCS encoder | HostPath/file open、source admission、trusted package issuance |
| `typaxis-machine-input` | package-root policy、package/source budgets、raw/canonical identityのsession binding、`AdmittedMachinePackage` | contained-open再実装、AST semantic validation、layout、manifest record構築 |
| `typaxis-syntax` | decoded DTO lowering、actual source/text/document/style/resource validation、entry-only closure、`ValidatedMachinePackage` issuance | host path解決、caller DTOのpublic trusted promotion |
| `typaxis-machine-profile` | immutable profile descriptor、host availability composition、canonical capability JSON、NodeId順preflight、profile/package-bound receipt | resource read、layout、PDF object生成 |
| `typaxis-cli` | option解決、phase orchestration、stderr、terminal publicationの起動 | JSON/domain invariant、capability feature list、trusted recordの再実装 |

Accepted dependency edges（既存core/domain edgeは省略）:

```text
typaxis-host-admission      -> typaxis-core
typaxis-document-package    -> typaxis-core
typaxis-machine-input       -> typaxis-core + typaxis-host-admission
                               + typaxis-document-package
typaxis-syntax              -> typaxis-document-package + typaxis-machine-input
typaxis-machine-profile     -> typaxis-core + typaxis-syntax + typaxis-diagnostics
typaxis-resource-admission  -> typaxis-host-admission
typaxis-manifest            -> typaxis-host-admission + typaxis-machine-input
                               + typaxis-syntax + typaxis-machine-profile
typaxis-cli                 -> typaxis-document-package + typaxis-machine-input
                               + typaxis-syntax + typaxis-machine-profile
                               + existing resource/layout/display/pdf/manifest crates
```

追加の禁止edgeとtrust rule:

- `typaxis-machine-input -> typaxis-syntax`。syntaxだけがprivate `ValidatedParsedPackage`を発行し、cycleと第二promotion pathを防ぐ。
- `typaxis-host-admission -> typaxis-document-package|typaxis-machine-input|typaxis-syntax|typaxis-resource-admission`。generic host receiptはlogical package/resource意味を知らない。
- `typaxis-document-package -> typaxis-host-admission|typaxis-machine-input|typaxis-syntax`。portable decodeはhost authorityやtrusted syntaxを発行しない。
- `typaxis-machine-profile -> typaxis-resource-admission|typaxis-layout|typaxis-display-list|typaxis-pdf`。preflightはresource bytesやoutputを生成しない。
- `typaxis-machine-input`、`typaxis-resource-admission`、CLIはcomponent walker、same-handle stable read、identity ledgerを複製しない。
- `WireDocumentPackage`はpublic DTOでありuntrusted。decoder-issued `DecodedDocumentPackage`、session-bound package/source receipt、`ValidatedMachinePackage`、capability/publication receiptはowner発行でprivate field、non-Clone、no raw-parts constructorとする。
- `typaxis-cli`はsealed progressをmanifest/diagnostics ownerへ渡すだけで、DTOやerror stringからtrusted factsを再構成しない。
