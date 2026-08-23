# Source、TextStore、Parser契約

`SourceCatalog`はadmit済みUTF-8 source bytes、`PortablePath`、byte length、SHA-256を所有する。`TextStore`はparser/normalizerが後段へ渡す正確なUnicode text bufferとtext mapだけを別所有する。`ParsedPackage`は`sources: SourceCatalog`と`text_store: TextStore`を別fieldで保持し、SourceIdとTextBufferIdのidentifier space、lifetime、offsetを共有しない。DocumentはTextStoreを所有せず`TextSpan`だけで参照する。

canonical allocationではentry sourceを`SourceId = 0`とし、include directiveをsyntax上の決定的depth-first順で辿ったfirst encounterごとに1ずつ割り当てる。`sources` arrayはこの0-based dense ID順で、URI順へsortしない。`TextBufferId`はdocs/04のtyped Document preorderでbufferを最初にemitした順に0から割り当て、`text_buffers` array indexと一致させる。worker completion、filesystem enumeration、hash-map iterationを割当順に使わない。include traversalとbuffer初回emitをportable artifactだけから再証明できない場合も、in-process constructorでこの規則を必ず検査する。

`SourceSpan`は原source、`TextSpan`はparsed `TextStore`の`TextBuffer`を指す。CRLF正規化、escape展開、syntax-time inserted textは`TextMapSegment`で対応付ける。pagination stateに依存するreference/counter/marker textはparsed `TextStore`へ追加せず、docs/05の`GeneratedTextStore`へ置く。

```text
identity    text byte lengthとsource byte lengthが等しく、対応する全bytesが同一でlocal offsetが1:1
replacement 正規化・escape展開など。長さ一致を要求しない
inserted    source spanを持たないgenerated text
```

segmentは非空で、buffer全体を隙間なく覆い、順序・重複・UTF-8境界を検証する。identity segmentはさらに両rangeのbyte lengthとbytesの一致を検証する。空segmentは同じmappingの非canonicalな別表現になるため拒否する。Unicode正規化は既定で行わない。NFC等を行う場合は明示設定とreplacement mapが必要。

URI-valued syntaxはsyntax境界でのみraw stringとして扱い、configured allowlistに含まれるASCII lowercase scheme、control/whitespace/NUL不在、raw UTF-8 lengthが`max_uri_bytes`以内であることを検査して`SafeUri`へ変換する。後段はraw URIを再解釈しない。

Parserの公開結果は`ParseOutcome`であり、`Parsed { package: Box<ValidatedParsedPackage>, diagnostics: Vec<AdvisoryDiagnostic> }`または`Failed { diagnostics: Vec<Diagnostic> }`のどちらかだけを表せる。`AdvisoryDiagnostic`はnote/warningだけを構築できる。`Failed`は少なくとも1件のerrorまたはfatalを含み、packageを持たない。fatal検出時は直ちに解析を打ち切り、errorは安全なparser境界まで追加diagnosticを収集できるが`Parsed`へ変換できない。`Parser` implementationはsyntax crate内にsealedし、`ValidatedParsedPackage`のinner fieldとowner constructorもprivateにする。featureを有効にしてcaller-authored `ParsedPackage`やfixture modelをtrusted packageへ昇格するAPIは持たない。回復ASTを返す場合もunknown IDや壊れたtext mapを後段へ流さない。

Parserはtyped nodeへdescendする前に`max_ast_nesting_depth`をconsumeし、Document rootをdepth 1、各Block、Inline、ListItem、TableRow、TableCell、FootnoteDefinitionをchild depth + 1として扱う。profile 1.0では設定値自体を64以下に制限する。syntax crate内のowner validationも、再帰するsource/text validation、node indexing、fingerprint計算より前に同じ木を反復走査し、max+1 nodeを再帰処理へ渡さない。StyleSheetの`extends`はroot ruleをdepth 1とする別のchainへ同じ上限を適用し、unknown parent/cycleはdepth errorへ潰さず専用errorで拒否する。

IncludeResolverはentryをdepth 0とし、各include edge、canonical path、discovery順、最大depth、source closureを持つ`ValidatedIncludeGraph` receiptを発行する。cycle、root外、深さ、file数、総bytesはfileの追加open/read前に検査する。`ValidatedParsedPackage`はこのreceiptを必須inputとし、flatなSourceCatalogだけを渡してinclude graphと`max_include_depth`の検査を迂回できない。
