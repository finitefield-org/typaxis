# Rust workspace境界

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
