# Machine input PDF統合の不足機能・文書改善計画・実装設計

## 1. 文書情報

- 状態: M0/M1実装・actual-host gate完了、M2〜M5実装設計
- 調査基準: Typaxis commit `6d9be4e20fb02901b5ff4f1bf4ef36643f4fd9e8`
- 設計基準: Typaxis commit `d11e90b03cac20435278fecef2fa8774b758ffad`
- 調査日: 2026-08-25
- M1完了確認日: 2026-08-26
- 検証host: `aarch64-apple-darwin`、Rust `1.97.1`
- 対象: CLI、canonical `DocumentPackage`、source/include trust、layout、Display、PDF、manifest、diagnostic、関連文書
- 主な利用者: Typaxisへ文書を機械生成して渡すproducer。特に、VMBのように自前のASTを持つ上流システム

本書は、調査基準commitの到達範囲を調査した結果と、machine-readableな文書入力からPDFを生成できるようにするための改善要求をまとめる。M0/M1の実装・完了記録は[task plan](25-machine-input-pdf-improvements-todo.md)を正とし、本書の調査結果は当時のgapを示すhistorical baselineとして保持する。M2以降は未実装箇所を明示するstatus/gap文書である。

## 2. 結論

調査基準commitのTypaxisは、machine inputからPDFを生成できなかった。現在のM1実装は、公開`build-package`/`check-package`/`capabilities`、sealed ingestion、`typaxis.machine-pdf/paragraph-1`のfont付きPDF経路を提供し、macOS/Linux actual-host evidence集約まで完了している。

`typaxis build INPUT`は引き続きINPUTをUTF-8のreference TSFとして読み、sealed `ReferenceParser`へ渡す。JSONの`DocumentPackage`は別の公開package commandがbounded decoder、source admission、trusted receiptを経由して処理する。portable validatorと`dump-ast --format json`だけではtrusted ingestionにならないという調査時のtrust-boundary判断は維持する。

加えて、調査対象commitではmacOS上のclean locked build/checkが`typaxis-resource-admission`のplatform `cfg`不整合でコンパイル失敗していた。このbaselineはMI0-01で復旧し、MI1-06でmacOS contained-openへ置き換え、MI1-17でcurrent-sourceのfont付きactual-host gateを閉じた。source tree内の既存`workspace/target/debug/typaxis`だけを対象revisionの実装証拠にしない規則は維持する。

さらに、machine input decoderだけを追加してもVMB等の一般的な文書はPDFにならない。CLIから到達するlayout/fragment/display経路はtop-level paragraphとheadingに限定され、list、table、figure、page break、footnote、link annotation、画像描画はend-to-endで未実装である。数式、SVG/vector、document language、outline、tagged PDFはDocument/PDFのportable contract自体にも不足がある。

したがって、次の三層を別々の完了条件として扱う必要がある。

1. **trusted machine ingestion**: JSON、companion source bytes、source closureを検証して`ValidatedParsedPackage`を発行する。
2. **PDF capability preflight**: schema-validだが現在のPDF経路で表現できないnode/resource/semanticsをlayout前に安定diagnosticで拒否する。
3. **general PDF pipeline**: rich Documentをflow、pagination、Display、resource finalization、PDFへlosslessに運ぶ。

初期実装では、1 sourceかつparagraph/heading中心の明示的なmachine profileだけを受理し、それ以外をfail closedにする。その後、capability profileを拡張する。full `DocumentPackage`を受理したように見せて深いlayout phaseで`L5000`へ落とす実装は採用しない。

## 3. 用語とstatus

本書では次のstatusを使う。

| status | 意味 |
|---|---|
| E2E | 公開CLI入力からPDF成果物まで実行可能で、integration testがある |
| 部分実装 | Rust type、validator、内部receipt、または一部phaseだけに実装がある |
| 契約のみ | docs/Schema/ADRに規定があるが、公開CLIから到達できる実装がない |
| 未実装 | 必要なwire model、公開command、またはphase実装がない |

「machine input」は、reference TSFではなく、versioned `DocumentPackage` JSONと、それが参照するsource/resource bytesを入力する経路を指す。「Schema-valid」はportableな構造検査に通ることだけを意味し、trusted packageやPDF-buildable packageであることを意味しない。

Findingのpriorityは次の意味で使う。

| priority | 意味 |
|---|---|
| P0 | trusted machine-PDF interfaceまたはfail-closedなsubset公開を阻む。最初のmachine input release前に必要 |
| P1 | production book profileまたはfull DocumentPackage対応を阻む。限定subsetは明示拒否を条件に先行可能 |
| P2 | 主にproducerの利用性・保守性を阻む文書/fixture不足 |

## 4. 現行実装の事実

### 4.1 現在のbuild経路

```text
typaxis build INPUT
  -> typaxis-cli::cli::parse_build
  -> typaxis-cli::main::run_build
  -> typaxis-cli::pipeline::load_package
       -> regular-file stable read
       -> UTF-8 decode
       -> preflight_reference_limits
       -> typaxis_syntax::ReferenceParser::parse
       -> ValidatedParsedPackage
  -> admit_resources
  -> layout_reference
       -> layout_paragraphs
       -> CanonicalFlowIrBuilder
       -> ReferencePaginator / ReferenceFragmenter
  -> ValidatedDisplayDocument::paint_reference_paragraphs
  -> ReferenceResourceFinalizer
  -> PdfBackend
```

根拠:

- `workspace/crates/typaxis-cli/src/cli.rs`のcommand集合は`build`、`check`、`dump-ast`、`dump-layout`、font inspectionだけである。
- `workspace/crates/typaxis-cli/src/pipeline.rs::load_package`は入力をUTF-8 textへ変換し、常に`ReferenceParser`を呼ぶ。
- `workspace/crates/typaxis-syntax/src/lib.rs::ReferenceParser`は小さなrecord grammarだけを実装する。
- `typaxis-cli`と`typaxis-syntax`には、DocumentPackage JSONをdecodeする依存・module・APIがない。

### 4.2 `dump-ast`は一方向のexport

```text
reference TSF
  -> ReferenceParser
  -> ValidatedParsedPackage
  -> artifacts::document_package_json
  -> DocumentPackage JSON

DocumentPackage JSON
  -X-> build/check/layout/PDF
```

`workspace/crates/typaxis-cli/src/artifacts.rs::document_package_json`はdecoderではない。さらに、style rule、page selection rule、font、image、footnoteを持つpackageを拒否し、block/inlineにもreference encoder固有の制限がある。このため、`dump-ast`という名称から期待される一般的なround tripは成立しない。

### 4.3 Schema検証とtrusted package発行は別の境界

`DocumentPackage.sources`が持つのは`source_id`、portable URI、byte length、SHA-256であり、source bytesそのものではない。一方、Rustの`SourceCatalog`はadmit済みsource bytesを所有し、`ValidatedParsedPackage::new_resolved`は次を検査する。

- source spanのowner、bounds、UTF-8 boundary
- identity TextMapのsource/text byte-for-byte一致
- Document node、anchor、footnote、resource参照
- style、page master、limit
- source closureに一致する`ValidatedIncludeGraph`

portable Schema validatorはsource bytesを持たないため、source code-point boundaryとidentity-map byte equalityを完結できない。この制約は`schemas/README.md`にも記載されている。したがって、SchemaをCLIから呼ぶだけではtrusted ingestionにならず、公開`ParsedPackage -> ValidatedParsedPackage` constructorを追加することも既存のsealed parser契約に反する。

### 4.4 include/source closureはwireから復元できない

`docs/03-source-text-and-parser.md`は、include edge、discovery順、depth、source closureを持つ`ValidatedIncludeGraph`を要求する。しかし、現行`document-package.schema.json`にはentry source IDやinclude edgeがない。flatな`sources` arrayからはdepth-first discovery順、cycle不在、include depthを証明できない。

machine inputの初期profileを1 sourceに限定するなら、`sources.length == 1`かつ`source_id == 0`を要求し、syntax ownerがentry-only closure receiptを発行できる。複数sourceを受理するなら、contractへsource graphを追加するか、別のversioned source-closure artifactを必須にしなければならない。

### 4.5 PDFへ到達するdomainはreference paragraph中心

現在の下流制限は次のとおりである。

- `pipeline::layout_paragraphs`はtop-level `Paragraph`と`Heading`以外を明示拒否する。
- CLIの`build_reference_flow`はparagraph itemだけをpushし、list item、table row、figure、page break boundaryを構築しない。
- `ReferenceFragmenter::for_paragraphs`もtop-level paragraph/heading以外を拒否する。
- `paint_reference_paragraphs`はglyph runを描画するが、page annotationを常に空で生成する。
- figure/imageをDisplayの`DrawImage`へ変換するpaint経路がない。
- `admit_resources`とlate finalizerはPNGを扱えるが、画像を使うDocumentがpaintへ到達しない。
- image admissionはPNGだけであり、JPEG/SVG decoderはない。
- font PDF profileはTrueType `glyf` outlineだけで、OTF/CFFは拒否する。

内部typeにlist/table/figure用boundaryやDisplay commandが存在することは、CLI E2E対応を意味しない。

### 4.6 現行support matrix

| 機能 | Rust/Schema | portable validation | trusted machine ingestion | 現行CLI PDF | 判定 |
|---|---:|---:|---:|---:|---|
| reference TSF paragraph/text | あり | N/A | N/A | あり | E2E |
| DocumentPackage JSON入力 | Rust model/Schemaあり、decoderなし | あり | なし | なし | 契約のみ |
| paragraph | あり | あり | なし | reference parser経由のみ | 部分実装 |
| heading | あり | あり | なし | parserが生成しない | 部分実装 |
| list | あり | あり | なし | layout/fragmentで拒否 | 契約のみ |
| table | あり | あり | なし | layout/fragmentで拒否 | 契約のみ |
| figure | あり | あり | なし | paint経路なし | 契約のみ |
| page break | あり | あり | なし | layoutで拒否 | 契約のみ |
| footnote | あり | あり | なし | flow/displayで拒否 | 契約のみ |
| anchor/reference text | あり | あり | なし | reference grammar内で部分対応 | 部分実装 |
| internal/external link annotation | modelあり | あり | なし | painterがannotationを発行しない | 契約のみ |
| block style | 4 propertyのみ | あり | なし | paragraph text styleで部分使用 | 部分実装 |
| custom page master | あり | あり | なし | reference parserはA4固定 | 部分実装 |
| PNG | 宣言/admission/finalizerあり | あり | なし | figure painterなし | 部分実装 |
| JPEG | なし | なし | なし | なし | 未実装 |
| SVG/vector | なし | なし | なし | なし | 未実装 |
| inline/block math | なし | なし | なし | なし | 未実装 |
| TrueType `glyf` | あり | あり | N/A | あり | E2E制約付き |
| OTF/CFF | metadata inspectionのみ | N/A | N/A | PDF profileで拒否 | 未実装 |
| document language/outline | package modelなし | なし | なし | なし | 未実装 |
| tagged PDF | なし | N/A | N/A | なし | 未実装 |
| diagnostic JSON schema | あり | あり | N/A | emit optionなし | 契約のみ |
| machine capability discovery | なし | N/A | N/A | なし | 未実装 |
| package JSON identity in manifest | schema fieldなし | N/A | N/A | なし | 未実装 |
| multi-source include resolver | receipt契約あり | flat factsのみ | なし | なし | 契約のみ |
| macOS clean locked build | sourceあり | N/A | N/A | compile error | 未実装/blocker |

### 4.7 調査時の検証結果

| command | 結果 |
|---|---|
| `python3 schemas/validate.py` | 成功。7 schemas、positive/cross-bundle fixture、205 invalid fixtureを検証 |
| `cargo build --manifest-path workspace/Cargo.toml --package typaxis-cli --locked` | macOSで失敗。fallback `HostResourceFile`に`exact_length`がない |
| `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked` | macOSで失敗。上記に加えtest buildで`PathBuf`が`cfg`により未import |

`cargo test`、CLI version、blank PDF smoke testはcompile gateを通らないため未実行である。

## 5. Findings

### TMI-001 [P0] machine input commandとdecoderがない

**影響:** producerは`DocumentPackage`を生成してもPDFへ渡せない。reference TSFを独自拡張する以外の経路がなく、上流ASTの意味を保持できない。

**必要な改善:**

- 自動判別ではない明示command `build-package`と`check-package`を追加する。
- bounded JSON decoderとtyped untrusted DTOを実装する。
- unknown field、duplicate key、invalid number、nesting/byte limitをtyped package構築前に拒否する。
- decoder成功をtrusted package成功とみなさず、TMI-002のsealed validatorへ渡す。

### TMI-002 [P0] source bytesを伴うtrusted machine ingestionがない

**影響:** source hashだけを信用すると、SourceSpanのUTF-8 boundaryとidentity TextMapのbytes一致を検査できない。公開promotion APIを追加すると、caller-authored ASTがsyntax trust boundaryを迂回する。

**必要な改善:**

- package rootへbindしたsealed input-admission sessionを追加する。
- `sources[].uri`をroot-contained、no-follow、regular-file、stable same-handle readでadmitする。
- declared byte length/SHA-256と実bytesを照合する。
- admitted source closureとprivate decoded DTOをsyntax ownerが検査して初めて`ValidatedParsedPackage`を発行する。
- downstreamから呼べるraw `ParsedPackage -> ValidatedParsedPackage` constructorは追加しない。

### TMI-003 [P0] multi-source packageを証明するwire factsとreceipt種別がない

**影響:** 現行Schemaの複数`sources`をそのまま受け入れると、include graph、cycle、depth、canonical discovery順を再証明できない。

**必要な改善:**

- 初期machine profileではexactly one sourceだけを受理する。
- 1 source profileでは`SourceId = 0`を要求し、syntax ownerがentry-only closureを発行する。arbitrary source bytesへreference TSFのinclude keyword scanを適用しない。
- multi-source対応時は、sourceの意味を先に二択から決める。
  - Typaxis syntaxのinclude sourceなら、entry、include-directive SourceSpan、ordered edge、discovery ordinalを持つversioned `source_graph`を追加し、syntax ownerがdirective bytesとgraphを照合する。
  - producerのprovenance sourceでinclude syntaxを持たないなら、caller-authored graphを`ValidatedIncludeGraph`として信用しない。`ValidatedMachineSourceClosure`等の別sealed receipt、array順の意味、source count/total limitをcontractへ追加する。
- 現在`ValidatedParsedPackage`が具体的な`ValidatedIncludeGraph`を保持する点を、必要ならsealed `ValidatedSourceClosure::{ParsedIncludes, MachineProvenance}`へ一般化し、phase ownership、invariants、fingerprintを同時に更新する。

### TMI-004 [P0] JSON自体のsecurity/limit契約がない

**影響:** `max_ast_nesting_depth`はtyped ASTの上限であり、deep JSONをtyped modelへ変換する前のstack/memory DoSを防げない。一般的なJSON libraryのdefaultではduplicate keyを最後の値で上書きする可能性もある。

**必要な改善:**

- `max_document_package_bytes`と`max_json_nesting_depth`をEffectiveConfig、Rust、Schema、CLI override、testsへ追加する。
- UTF-8、BOM不許可、root object、duplicate key不許可、finite/integer/range、unknown member不許可を規定する。
- whitespaceとobject member orderは入力上許可し、typed modelをJCS再encodeしたsemantic hashを別に作る。raw input hashも保持する。
- depth/size/token budgetを再帰的deserializationより前に検査する。

### TMI-005 [P0] build manifestがmachine package本体をbindできない

**影響:** 現行manifestの`inputs`は`SourceCatalog`だけから作られる。source filesが同じでもAST/style/page masterを持つpackage JSONが変わればPDFは変わり得るため、machine buildの再現性identityが欠落する。

**必要な改善:**

- manifestへ`input_profile`とnullable `package_input`を追加する。
- `package_input`はpackage-root-relative URI、raw byte length、raw SHA-256、判明後のcontract IDとtyped canonical JCS SHA-256を持つ。
- source modeは`input_profile = "typaxis.reference-source/1"`かつ`package_input = null`を要求する。built machine modeはversioned machine profileと完全なnon-null recordを要求し、failed machine modeはadmit済み段階までのnullable fieldだけを持つ。
- failed manifestも、admit済み段階までのpackage/source factsだけをsealed ownerから記録する。

### TMI-006 [P0] schema-validとPDF-buildableを分けるcapability gateがない

**影響:** decoderだけを追加すると、schema-validなlist/table/figure等が深い`L5000`で失敗するか、意味を落として描画される危険がある。producerは実行前に利用可能なfeatureを判定できない。

**必要な改善:**

- `ValidatedParsedPackage`発行後、resource read/layout前に`MachinePdfCapabilityPreflight`を実行する。
- NodeId preorderでunsupported featureを決定的に収集し、上限内の全件をstable diagnosticとして返す。
- 1 executionのdiagnostic materializationをfixed `MAX_MACHINE_DIAGNOSTICS = 256`でboundedにし、超過分を最後のrecordのnoteへ集約する。
- capability IDを`typaxis.machine-pdf/paragraph-1`のようにversion化し、manifestへ記録する。
- `typaxis capabilities --format json`でblock、inline、style、resource、font、PDF feature、source closure profileをmachine-readableに公開する。
- capability outputと実装のE2E fixtureを同じchange setで更新する。

### TMI-007 [P1] general block layout/fragmentationがCLI経路にない

**影響:** machine parserがrich ASTを作れてもparagraph/heading以外をPDFへ送れない。

**必要な改善:**

- Document typed preorderを走査してparagraph、list item、table row、figure、page breakの全canonical flow boundaryを構築する。
- list marker、nested blocks、table cell subflow、figure/caption、page breakを個別のvalidated layout receiptへ変換する。
- `ReferenceFragmenter`とは別に、対応domainを名前とtypeで明示したproduction fragmenterを追加する。
- footnote、column、floatは本文flowと別cursor/terminal/receiptを持ち、body continuationへ混在させない。
- blockごとのexact-limit、progress、pagination、selected-state paint testsを追加する。

### TMI-008 [P1] link、figure、imageのpaint経路がない

**影響:** `Inline::Link`をtextだけとして描くとlink semanticsが失われる。figureを受理しても`DrawImage`へ到達しない。

**必要な改善:**

- inline link rangeからselected page上のannotation rectangleを導出する。
- internal linkはpackage anchor closureとexact一致するnamed destinationへ、external linkはadmit済み`SafeUri`へ束縛する。
- figure/image placementから`DrawImage`を発行し、Display usage、admitted ledger、late finalizerを同じImageResourceIdへbindする。
- annotation/destination/imageのmissing、extra、wrong page、wrong resource negative testsを追加する。

### TMI-009 [P1] VMB相当の文書を表すmodel/style/resourceが不足する

**影響:** machine ingestionが完成しても、結果・証明・演習の複数block container、数式、vector asset、book metadataをlosslessに表現できない。

不足項目:

- generic section/container/admonition相当のblockと、その子block ownership
- inline mathとdisplay math
- math source、rendered vector、読み上げtextの対応関係
- SVGまたはbackend-neutral vector display resource
- imageのmedia typeとintrinsic size
- margin、padding、indent、alignment、color、border、background、width、keep、widow/orphan等のtyped style
- title、author等のdocument metadata、BCP 47 language、outline hierarchy
- semantic heading/result/proof/exerciseをPDF structureへ伝える情報

上流producerのunknown nodeをparagraphへ平文化したり、数式をraw TeX/plain text/rasterへ暗黙fallbackしたりしない。専用modelを追加するか、意味を保存するversioned lowering contractを先に定義する。

### TMI-010 [P1] resource profileがproduction book用途に不足する

**影響:** 現在はTrueType `glyf`とPNGの一部だけで、一般的なOTF/CFF、JPEG、SVG/vectorを扱えない。さらにPNGもfigure painterがないためE2Eでは使えない。

**必要な改善:**

- resource declarationへclosed media typeを追加する。
- JPEGを実装するなら`docs/21-roadmap.md`のtargetだけでなくadmission、decoded metadata、Display usage、PDF encoding、manifest、fixturesを同時に実装する。
- SVGは外部参照、script、font/network依存を禁止したsafe subsetを定義するか、admit済みvector display IRへ変換する。
- OTF/CFFはFontFile3/CIDFontType0C等の別PDF planとして実装し、TrueType FontFile2へ流さない。
- formatごとにbytes、pixels/vector commands、nesting、decoded memory、object count limitを追加する。

### TMI-011 [P1] structured machine diagnosticの公開経路がない

**影響:** `diagnostics.schema.json`はあるが、CLIはcanonical diagnostic sidecarを出力しない。package JSON decode errorにはSourceSpanがまだ存在しないため、producerが安定位置を取得できない。

**必要な改善:**

- `--emit-diagnostics PATH`を`build-package`と`check-package`へ追加する。
- diagnostic locationへ`package_json`（portable URI、JSON Pointer、byte offset）と`source`（SourceSpan）を区別するtagged unionを追加する。
- stderrは人向け、sidecarはcanonical JCSの機械向けとし、code/severity/locationの意味をversion化する。
- successはnote/warningだけ、error/fatalはartifact successなしという既存契約を維持する。
- output/trace/manifest/diagnostics targetのalias、atomic publish、failure時の扱いを一つのexecution contextで定義する。

### TMI-012 [P1] PDF publication semanticsがbook用途に不足する

**影響:** document language、outline、tagged structure、math accessibilityがないため、production/release PDFのnavigation/accessibility要件を満たせない。

**必要な改善:**

- Catalog `/Lang`、outline tree、heading/section destinationのownershipをDocumentから導出する。
- tagged PDFを実装する場合はDocument semantic nodeとmarked-content/structure elementをreceiptで束縛する。
- ActualTextだけをtagged PDFの代用としない。
- feature未実装時はcapability outputとmanifestへ明示し、release profileを受理しない。

### TMI-013 [P0] 調査対象HEADがmacOSでclean buildできない

**影響:** 現在のsource revisionからCLI binaryを作れず、既存TSF PDFもmachine input実装も同revisionで検証できない。`workspace/README.md`のbuild/check/test手順と`docs/23`のchecked `cargo check/test`表示が実際のmacOS結果と矛盾する。古い`workspace/target/debug/typaxis`を誤って使う危険もある。

**根拠:**

- `HostResourceAdmissionSession::open_font`と`open_image`は全platformで`reader.exact_length()`を呼ぶ。
- `HostResourceFile::exact_length`はAndroid/Linux `cfg`側にしかなく、macOS fallback typeには存在しない。
- all-targets test buildは`PathBuf`を使う一方、そのimportがAndroid/Linux `cfg`内にある。
- `ConfigResourceRoot`もmacOS buildでunused importとなり、clippy `-D warnings` gateを通らない。

**必要な改善:**

- unsupported contained-open platformでもtype-checkでき、resource open時に`UnsupportedContainedOpen`へfail closedするAPIへ整理する。到達不能を理由に架空のlengthを返さない。
- platform-specific importとtest helperの`cfg`を実装domainに合わせる。
- M0 baselineではmacOSでbuild/check/test/clippyとblank PDF smoke testを実行し、resourceを要求するfixtureがruntimeのstable unsupported errorになることを検証する。M1公開時は12.3のcontained resource openへ置き換える。
- documented support matrixにcompile support、atomic output support、contained resource-open supportを別列で記録する。
- source revision、binary `--version`、binary digestをintegration evidenceへbindする。

### DOC-001 [P1] 「現行仕様」と「参照実装の到達範囲」が混同される

変更前の`README.md`は文書が現行仕様を記述すると述べ、`docs/23-implementation-checklist.md`は全項目をcheckedにしていた。一方、`workspace/README.md`はcompleted engineではなくbounded reference parser/layout domainであると明記する。

契約invariantがRust type/Schema/testで固定されていることと、公開CLIでfeatureがE2E実装されていることは別のstatus axisである。本変更でREADME、roadmap、checklist、contract matrix、CLIに相互参照を追加し、今後は本書のsupport matrixを到達性の正本とする。

### DOC-002 [P1] CLIのINPUT形式と`dump-ast`の非round-trip性が不明瞭

変更前の`docs/19-cli.md`では`INPUT`が形式名を持たず、`dump-ast`が出すJSONを`build`へ渡せないことも記載されていなかった。現行commandはreference TSF専用、JSONはexport/fixture artifactであることを明示する必要がある。

machine input実装後は、reference source guideとmachine package guideを分離し、copy-and-pasteできる成功例、directory layout、expected manifest/diagnosticを追加する。

### DOC-003 [P1] roadmap/checklistが実装statusとして読める

`docs/21-roadmap.md`のM1にはJPEGがあるが、現行resource admissionはPNGのみである。変更前はroadmapに完了statusの説明がなく、`docs/23`のchecked itemと合わせると、目標・contract test・runtime completionの区別がつかなかった。

roadmapはtarget順、checklistはcontract invariant、support matrixはCLI E2E statusとして役割を固定する。milestoneを完了扱いにする場合は、該当capabilityのCLI fixtureとsupport matrix更新を必須にする。

### DOC-004 [P2] machine producer向けguideとfixtureがない

不足する説明:

- package rootとcompanion source/resource配置
- SourceId、NodeId、TextBufferIdのcanonical割当例
- source bytes/hashとidentity TextMapの作り方
- accepted/rejected capability profile
- package/manifest/trace/diagnosticのhash関係
- duplicate key、stale source hash、unknown capability等のnegative example
- `dump-ast -> build-package` round-trip保証範囲

実装時に`samples/machine-package/`へ最小blank、paragraph/heading、invalid provenance、unsupported capability fixtureとREADMEを追加する。

### DOC-005 [P2] CLI binaryのbuild/run/version確認手順が不足する

`workspace/README.md`にはcheck/test/clippy/fmtはあるが、CLI binaryをbuildして実行する最短手順がなかった。`docs/19-cli.md`もcommandのargument契約が中心で、source revisionより古い`workspace/target/debug/typaxis`を誤って使わないためのversion確認手順がなかった。

本変更でlocked build、`--version`、blank PDF smoke testを`workspace/README.md`へ追加し、`docs/19-cli.md`のcommand一覧にもhelp/versionを追加する。release producerはsource tree内の既存binaryを暗黙採用せず、対象revisionからbuildしたbinaryまたは明示されたrelease artifactのversion/digestをadmitする。

## 6. 推奨するmachine input契約

### 6.1 CLI

```text
typaxis build-package PACKAGE.json -o OUTPUT.pdf \
  [--package-root DIR] \
  [--profile typaxis.machine-pdf/paragraph-1] \
  [--config CONFIG] [--resource-root DIR ...] [--strict] \
  [--trace TRACE.json] \
  [--emit-build-manifest MANIFEST.json] \
  [--emit-diagnostics DIAGNOSTICS.json]

typaxis check-package PACKAGE.json \
  [--package-root DIR] \
  [--profile typaxis.machine-pdf/paragraph-1] \
  [--config CONFIG] [--resource-root DIR ...] \
  [--emit-diagnostics DIAGNOSTICS.json]

typaxis capabilities --format json
```

決定事項:

- `build`はreference TSF、`build-package`はDocumentPackageとし、extension/content sniffingをしない。
- 初期releaseのmachine profileは`typaxis.machine-pdf/paragraph-1`だけとする。`--profile`省略時もこのIDへ解決し、resolved IDをmanifestへ記録する。producerは再現可能なjobでは明示指定する。unknown IDを最新版へ暗黙fallbackしない。
- `check-package`成功はSchema shapeだけでなく、trusted source validation、現在のPDF capability preflight、resource admission、computed style/font family解決成功を意味する。pagination、full glyph shaping、PDF serializationまでは実行しない。
- `build-package`は既存buildのstrict、trace、manifest、compression、limit、atomic output規則を共有する。
- unsupported machine featureからreference TSFや別backendへfallbackしない。

### 6.2 directory layout

```text
job/
  document-package.json
  sources/
    book.json
  fonts/
    body.ttf
  images/
    cover.png
```

例:

- `PACKAGE.json = job/document-package.json`
- default `package-root = job/`
- `sources[0].uri = "sources/book.json"`
- `resources.font_faces[0].uri = "fonts/body.ttf"`

`--package-root`省略時はPACKAGEのparent directoryを使う。明示時はPACKAGE自体もroot-containedであることを要求し、manifestへroot-relative URIを記録する。package rootはHostPath execution contextだけが所有し、canonical artifactへabsolute pathを保存しない。

sourceはpackage rootだけから解決する。font/imageは既存のadmitted resource-root集合から解決できるが、0 candidateはmissing、2以上はbytesが同じでもambiguousとする。package rootはresource rootへ暗黙追加しない。producerがprivate job directoryだけを公開したい場合は、`--resource-root job/`または同等configを明示してpackage rootを唯一のresource rootにする。

### 6.3 JSON受理規則

- UTF-8のみ。BOM、raw NUL byte、trailing tokenを拒否する。
- object duplicate keyを全depthで拒否する。
- Schemaの`additionalProperties: false`と同じunknown field拒否をtyped decoderでも行う。
- integer fieldはJSON integer lexical formと型の上限を要求し、floatからの暗黙変換をしない。
- raw whitespaceとobject member orderは許可する。
- arrayのcanonical order、dense ID、class/source-order等はsemantic validatorでexactに検査する。
- raw bytesのSHA-256と、typed modelをJCSへ再encodeしたcanonical SHA-256を区別する。
- JSON parse depth、typed AST depth、source bytes、text bytes、resource bytesは別limitとして検査する。

### 6.4 source provenance

初期profile:

- `sources.length == 1`
- `sources[0].source_id == 0`
- declared URI、length、SHA-256がadmit済みcompanion fileと一致
- 全SourceSpanがsource 0のUTF-8 boundary内
- identity TextMapがsource bytesとbyte-for-byte一致
- replacement/inserted mappingが既存contractどおり

VMB等のproducerは、元project rootをTypaxisへ公開する必要はない。private job directoryへ、source provenanceの正本となるcanonical producer inputを1 fileだけstageし、全spanをそのfileへ向けられる。

複数source対応を追加するまで、`sources.length > 1`は専用stable codeで拒否する。flat arrayから架空のinclude edgeを作らない。将来producer provenanceを複数fileへ分ける場合は、既存include depthを流用せず、machine source count/total bytesとcanonical orderingを持つ別closure profileを追加する。

### 6.5 crate/trust boundary

```text
typaxis-cli
  -> package/output HostPath options
  -> sealed machine-input package admission
       -> stable package bytes
       -> package-root capability
  -> typaxis-document-package
       -> bounded strict JSON decode
       -> decoder-issued untrusted DTO receipt
  -> sealed machine-input source admission
       -> stable companion source bytes
  -> typaxis-syntax::DocumentPackageParser
       -> source/text/document/style/resource validation
       -> entry-only receipt
       -> ValidatedMachinePackage { ValidatedParsedPackage, provenance }
  -> capability preflight
  -> existing admitted resources/layout/display/PDF
```

推奨type:

```rust
pub struct AdmittedMachinePackage {
    /* private: root/session identity, package bytes, admitted source closure */
}

pub struct DocumentPackageParser {
    /* sealed */
}

impl DocumentPackageParser {
    pub fn parse(
        self,
        input: AdmittedMachinePackage,
        policy: &PackageValidationPolicy<'_>,
    ) -> MachineParseOutcome;
}
```

実際のfield/API名は実装時に確定してよいが、次の性質は必須である。

- callerはadmission receiptを組み立てられない。
- callerはdecoded DTOや`ParsedPackage`をtrusted packageへ直接promoteできない。
- package bytesとsource bytesの同一session identityを検査する。
- parse成功値は既存`ValidatedParsedPackage`をprovenanceと一緒に所有するwrapperであり、下流へ別の弱いAST typeを増やさない。

### 6.6 検査順と副作用境界

同じ入力が同じprimary errorと副作用を持つよう、順序を固定する。

1. CLI syntax/unknown profileとconfigを検査し、write target相互および既知のPACKAGE/configとのaliasを拒否してpublication contextを作る。
2. compiled host capabilityを検査する。
3. package rootとPACKAGEをcontained regular fileとしてstable-openする。
4. package byte limit、UTF-8、JSON lexical/depth/duplicate-keyを検査する。
5. contract、coordinate unit、root fieldをtyped decodeする。
6. source catalogの形、ID、URI、declared length/hashをpreflightする。
7. companion sourceをstable-openし、bytes/hash/UTF-8をadmitする。
8. source closure、SourceSpan、TextMap、node/style/master/resource semanticsを検査する。
9. `ValidatedParsedPackage`を発行する。
10. 全safe font/image URIについて、全resource rootから導出されるread candidateをopenせずread ledgerへ登録し、write targetとのaliasを拒否する。
11. machine PDF capabilityをNodeId順にpreflightする。
12. 許可されたfont/imageをadmitし、computed style/font family解決を検査する。
13. layout、pagination、Display、resource finalization、PDF graphを作る。
14. PDF、trace、manifest、diagnosticsを既存atomic publication contractでcommitする。

step 11までunsupported contentのためにresource bytesやPDF temp fileを書かない。ただしstep 10のcandidate登録は、unsupported resource宣言を含むfailure sidecarが入力候補を上書きしないために必須であり、resource file自体はopenしない。step 3以後にmanifest/diagnostics targetが成立した場合のfailed sidecar規則は、既存buildのterminal publication contractへ統合する。

### 6.7 capability artifact

`typaxis capabilities --format json`は少なくとも次を持つ。

```json
{
  "contract": "typaxis.contract/1.1",
  "engine": {
    "name": "typaxis",
    "version": "0.1.0"
  },
  "machine_input": {
    "coordinate_units": ["pdf_point_1_65536"],
    "default_profile": "typaxis.machine-pdf/paragraph-1",
    "document_package_contracts": ["typaxis.contract/1.0", "typaxis.contract/1.1"],
    "host_features": {
      "atomic_file_publish": true,
      "contained_package_open": true,
      "contained_resource_open": true
    },
    "host_limits": {
      "max_read_candidates": 131072,
      "max_resource_roots": 64
    },
    "limits": {
      "max_document_package_bytes": {
        "default": 134217728,
        "maximum": 9007199254740991
      },
      "max_json_nesting_depth": {
        "default": 256,
        "maximum": 256
      }
    },
    "max_diagnostics": 256,
    "profiles": [
      {
        "available": true,
        "blocks": ["heading", "paragraph"],
        "font_formats": ["sfnt-truetype-glyf", "ttc-truetype-glyf"],
        "footnotes": false,
        "id": "typaxis.machine-pdf/paragraph-1",
        "image_formats": [],
        "inlines": {
          "kinds": ["anchor", "hard_break", "reference", "soft_break", "text"],
          "reference_formats": ["page"]
        },
        "page_master": {
          "count": 1,
          "optional_frames": [],
          "selection_rules": false
        },
        "page_values": ["auto"],
        "pdf_features": ["named-destinations", "text-extraction"],
        "source_closure": "entry_only",
        "source_count": {
          "maximum": 1,
          "minimum": 1
        },
        "style_block_types": ["heading", "paragraph"],
        "style_properties": ["font_family", "font_size", "line_height", "page"],
        "style_selectors": ["heading", "paragraph"],
        "unsupported_pdf_features": ["heading-semantics", "link-annotations", "outlines", "tagged-pdf"]
      }
    ]
  }
}
```

これは初期profileの形を示す例であり、現行CLIが出力できるという意味ではない。`sfnt-truetype-glyf`はstandalone sfnt、`ttc-truetype-glyf`はTTC内faceを表し、どちらもTrueType scaler + `glyf` outlineに限定する。実装時はprofileが約束する視覚/semantic outputをE2Eで満たすfeatureだけを列挙する。たとえば`link`はannotation生成まで、`figure`はimage paintまで、`emphasis`/`strong`は指定した視覚/semantic contractを保持できるまでadvertiseしない。

### 6.8 build manifest

推奨追加field:

```json
{
  "input_profile": "typaxis.machine-pdf/paragraph-1",
  "inputs": [
    {
      "bytes": 67890,
      "sha256": "<source-sha256>",
      "uri": "sources/book.json"
    }
  ],
  "package_input": {
    "bytes": 12345,
    "canonical_sha256": "<typed-jcs-sha256>",
    "contract": "typaxis.contract/1.1",
    "sha256": "<raw-package-sha256>",
    "uri": "document-package.json"
  }
}
```

source modeにも`input_profile`を必須にし、`package_input = null`とする。manifestのSchema、Rust type、JCS encoder、minimal/conformance/invalid fixtures、validator、docsを一つのchange setで更新する。

### 6.9 diagnostic location

machine packageのsyntax errorはSourceSpanを作る前に発生するため、次のようなlocation unionが必要である。

```json
{
  "byte_offset": 1942,
  "json_pointer": "/document/blocks/3",
  "kind": "package_json",
  "uri": "document-package.json"
}
```

source/text validation後のerrorは既存SourceSpan/TextSpan/NodeIdを使う。JSON PointerはRFC 6901 escapeを使い、array indexをdecimal canonical formにする。byte offsetを特定できないsemantic errorでもPointerは必須にし、host absolute pathは出さない。

### 6.10 round trip

machine input公開後、次を明示的に保証する。

```text
supported reference source
  -> dump-ast --format json
  -> build-package
  -> same validated DocumentFingerprint
```

`typaxis-syntax`は全current DocumentPackage 1.1 domain variantをexhaustive matchで`WireDocumentPackage`へ変換し、`typaxis-document-package`のJCS encoderだけがwire bytesを生成する。新domain variantにwire表現がない状態はcompile時または明示的なcontract migration errorで止め、CLI内に部分serializerやfield mappingを残さない。machine profileの受理範囲はこのwire変換範囲とは別にcapability gateが決める。raw JSON bytesの一致ではなく、typed canonical JCSとDocumentFingerprintの一致をround-trip条件にする。

## 7. VMB等のbook producerに必要な追加機能

machine ingestionだけをproduction backend完成条件にしてはならない。

| producer要件 | 現行Typaxis | 必要な改善 |
|---|---|---|
| chapter/section heading | modelはあるがmachine ingressなし | ingress、heading layout、outline |
| result/proof/exercise | grouping containerなし | generic semantic containerまたはlossless lowering contract |
| inline/display math | modelなし | math node、vector resource、source/speech mapping |
| list | model/validatorのみ | nested flow、marker、fragment/display |
| table | model/validatorのみ | cell subflow、row split、header repeat、paint |
| footnote | model/contractのみ | separate subflow、reflow、paint |
| figure/caption | modelのみ | placement、image/vector paint、caption flow |
| internal/external link | model/display typeあり | annotation paint、destination closure |
| SVG/PNG/JPEG asset | PNG admissionだけ | media type、figure E2E、SVG/JPEG |
| custom trim/page master | modelあり | machine ingress、E2E page selection |
| horizontal/vertical/RTL profile | bidi一部/horizontalのみ | writing mode capabilityとlayout |
| book navigation | named destination一部 | outline、document language |
| accessibility | ActualText一部 | tagged PDF、semantic structure、math alternative |
| structured error mapping | schemaのみ | emitted diagnostics sidecar |

初期の実験profileはparagraph/heading/textだけでもよい。ただし、未対応nodeをproducer側で黙って削除・平文化・rasterizeせず、Typaxisのcapability preflightかproducerの同一capability表でbuild前に拒否する。production/release profileは上表の必要subsetとaccessibility policyを自動testで満たした後にだけadvertiseする。

## 8. 実装順

### M0: statusと契約判断

Tasks:

- 本書をsupport/gapの正本として公開する。
- TMI-013を修正し、documented hostでclean locked build/check/test baselineを復旧する。
- machine input ADRを追加し、明示command、package root、single-source MVP、manifest identity、capability profileを採択する。
- README、CLI、roadmap、contract matrix、checklistのstatus axisを分離する。

完了条件:

- docsだけを読んだ利用者が、現行`build`へDocumentPackage JSONを渡せないことを誤解しない。
- contract-only、部分実装、CLI E2Eの区別が全関連文書で一致する。
- macOSでcurrent sourceからCLIをbuildでき、blank PDF smoke testが成功する。

### M1: trusted single-source ingestionとparagraph PDF

Tasks:

- stable package/source admissionを実装する。
- bounded duplicate-key-rejecting JSON decoderを実装する。
- `DocumentPackageParser`からsealed `ValidatedParsedPackage`を内包する`ValidatedMachinePackage`を発行する。
- `build-package`、`check-package`、`capabilities`を追加する。
- machine profileをparagraph/headingと実証済みinlineに限定する。
- package input identityをmanifestへ追加する。
- package JSON locationを持つdiagnostic sidecarを追加する。
- `dump-ast`の対応範囲をserializer/testで固定する。

完了条件:

- `samples/minimal/document-package.json`相当のsingle-source blank packageがPDFを生成する。
- font付きparagraph/heading fixtureが実PDF、manifest、trace、diagnosticsを生成する。
- source hash/length、identity map、duplicate key、unknown field、deep JSON、multi-source、unsupported blockをそれぞれ専用errorで拒否し、PDFを残さない。
- package/sourceをstable read中に変更したtestがfail closedになる。
- 同じpackage/root/config/resourcesの二重buildがPDFと全sidecarでbyte一致する。
- supported `dump-ast -> build-package`が同じDocumentFingerprintになる。

### M2: general flowとbasic document semantics

Tasks:

- list、page break、figure/captionのflow/fragment/displayを実装する。
- link annotationとnamed destinationを実装する。
- PNG figureをE2Eにする。
- typed block spacing、alignment、keep、indent等を追加する。
- capability profileとsupport matrixを実装済みfeatureだけ拡張する。

完了条件:

- 各featureにpositive E2E、unsupported/tamper negative、page split、exact-limit testがある。
- capability JSONに列挙した全featureを組み合わせたpackageがPDFまで成功する。
- unknown/unsupported featureをlayout開始前に拒否する。

### M3: table、footnote、advanced pagination

Tasks:

- table cell subflow、row fragmentation、repeated headerを実装する。
- footnote subflow/reflow/split policyを実装する。
- header/footer、column、floatを必要profileに従って実装する。
- selected-state Displayとtrace closureを全subflowへ拡張する。

完了条件:

- docs/10のexact-limitとprogress規則がCLI E2Eで検証される。
- table/footnoteを含むpackageがtrace、manifest、PDFで同じselected stateへbindされる。

### M4: math/vector/book publication

Tasks:

- inline/display mathとsafe vector resource contractを追加する。
- SVG/JPEG、必要ならOTF/CFFを独立profileとして実装する。
- semantic container、document metadata/language、outlineを追加する。
- tagged PDF/accessibility policyを実装する。

完了条件:

- 数式source、vector paint、text alternative、source spanの対応がtamper不能なreceiptで結ばれる。
- document language、outline、link、tagged structureを独立PDF validatorで検査する。
- VMB等のproduction fixture全体をlosslessに生成できる。

### M5: hardeningとrelease

Tasks:

- machine JSON/source/resource fuzzingを追加する。
- renderer/extractor/accessibility differential testを追加する。
- capability/manifest/trace/diagnostic tamper matrixを追加する。
- supported platform、resource governance、font license、tool identityをrelease policyへ結ぶ。

完了条件:

- same-toolchain reproducibility、limits、fuzz、differential、tamper gateが全て通る。
- release profileが未対応capabilityを含む場合、process/output開始前に拒否される。

## 9. 文書構成の改善

本変更では誤解防止のため次を行う。

- `README.md`: contractとreference CLI実装のstatusが別であること、本書へのlinkを追加する。
- `docs/19-cli.md`: 現行INPUTがreference TSF専用で、`dump-ast` JSONをbuildできないことを追記する。
- `docs/21-roadmap.md`: roadmapがtarget順でありcompletion recordではないことを追記する。
- `docs/22-contract-matrix.md`: machine package ingestionの未実装rowを追加する。
- `docs/23-implementation-checklist.md`: checked itemがcontract invariantの証拠でありCLI E2E completionではないことを追記する。
- `schemas/README.md`: offline validationがtrusted CLI ingestionではないことを追記する。
- `workspace/README.md`と`docs/19-cli.md`: CLIのlocked build、version、smoke-test手順を追加する。

machine input実装時には、さらに次を追加する。

- `docs/26-machine-input-cli.md`: producer向けnormative user guide
- `adr/ADR-0027-machine-document-package-ingestion.md`: trust/compatibility decision
- `samples/machine-package/README.md`: runnable bundleとnegative fixtures
- `contracts/machine-pdf-capabilities.md`: capability IDと互換性規則

番号・ファイル名は追加時のrepository stateに応じて調整できるが、内容をCLI helpだけへ閉じ込めない。

## 10. 「machine input対応済み」の判定条件

次をすべて満たすまで、README、release note、capability artifactでmachine input対応済みと表記しない。

1. documented targetでcurrent sourceのlocked build/check/testが成功する。
2. 公開`build-package`が存在し、DocumentPackage JSONからPDFを生成する。
3. companion source bytesをadmitし、SourceSpan/TextMap identityを検証する。
4. sealed syntax owner以外にtrusted package promotion pathがない。
5. current PDF capabilityを事前検査し、unsupported nodeをlosslessに拒否する。
6. manifestがraw/canonical package identityとsource/resource identityをbindする。
7. structured diagnosticsがpackage JSON PointerまたはSourceSpanを返す。
8. package/output/sidecar targetのstable read、alias、atomic publication規則がsource modeと同等である。
9. positive、negative、tamper、limit、二重build、round-trip E2E testがある。
10. support matrix、CLI、roadmap、checklist、samplesが実装と一致する。

「machine input対応済み」と「VMB等のfull bookをproduction PDFにできる」は同義ではない。後者には、少なくとも本書7章とM2〜M5のうち対象profileが必要とする項目の完了が必要である。

## 11. 実装設計の適用範囲

最初の公開単位はM0とM1を合わせた`typaxis.machine-pdf/paragraph-1`とする。本章以降では、この公開単位を実装者が追加判断なしで着手できる粒度まで具体化する。M2〜M5については、M1で固定するtrust boundary、flow registry、capabilityの拡張点を規定するが、各rich featureのwire contractと組版policyは対応milestoneのADRで別途固定する。

初回公開に含めないもの:

- multi-source machine package
- list、table、figure、footnote、link annotation
- emphasis/strongの暗黙なplain-text化
- remote source/resource fetch
- math、SVG/vector、JPEG、OTF/CFF
- outline、tagged PDF、release/book profile
- unsupported nodeのreference TSFまたはrasterへのfallback

M1の実装中にこれらを偶然通せても、capability artifactへ追加せずpreflightで拒否する。M2以降でprofileを拡張するときは、model、preflight、layout、Display、PDF、manifest、fixtureを同じchange setで更新する。

## 12. M0/M1の実装アーキテクチャ

### 12.1 crate境界

M1では四つのcrateを新規追加し、既存の`typaxis-syntax`と`typaxis-cli`も次表の責務へ更新する。

| crate | 所有する責務 | 禁止する責務 |
|---|---|---|
| `typaxis-host-admission` | directory handle/root set、contained open、stable file read、host read identity ledger | package/resource ID、JSON/domain decode、canonical artifact |
| `typaxis-document-package` | untrusted wire DTO、strict JSON preflight/decode、JSON Pointer index、DocumentPackage JCS encoder | host path、file open、trusted package発行 |
| `typaxis-machine-input` | package root policy、package/source read budget、raw identity、session binding、`AdmittedMachinePackage` | contained-open再実装、AST semantic validation、layout、manifest recordの直接構築 |
| `typaxis-syntax` | wire DTOからdomain typeへのlowering、source/text/document/style/resource validation、entry-only closure、`ValidatedParsedPackage`発行 | host path解決、任意DTOのpublic promotion |
| `typaxis-machine-profile` | profile descriptor、capability JSON、NodeId順preflight、package/profile-bound receipt | resource read、layout、PDF object生成 |
| `typaxis-cli` | option解決、phase orchestration、stderr、terminal publication | JSON/domain invariantの再実装 |

新規・変更dependency edgeは次に固定する。既存のcore/domain crateへのedgeは省略する。

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

`typaxis-machine-input`は`typaxis-syntax`へ依存しない。`typaxis-syntax`だけが既存のprivate `ValidatedParsedPackage` constructorへ到達できるため、crate cycleや別promotion pathを作らない。`typaxis-document-package`が公開するDTOは明示的にuntrustedであり、callerがDTOを組み立てられても`AdmittedMachinePackage`を発行できないことがtrust条件である。

security-sensitiveなcomponent walker、same-handle snapshot、stable bounded read、read/write alias identityをmachine/resource crateへ二重実装しない。`typaxis-host-admission`がgeneric receiptを発行し、machine/resource ownerがそれをlogical package/source/font/image IDと各budgetへbindする。`typaxis-host-admission`はraw host pathやfile handleからcanonical recordを作らない。

正確には、encode用`WireDocumentPackage`はcallerが構築できるが、decoder-issued `DecodedDocumentPackage`はprivate bindingを持ちcallerが構築・変更できない。trusted pathはdecoded valueを後付けでbindするpublic APIを持たず、machine admission sessionが自身のraw receipt上でdecoderを呼ぶ。これにより、export用DTOの再利用とdecode receiptの非偽造性を両立する。

`dump-ast`のencoderはCLI内の別実装を維持しない。domain-to-wire変換はdomain ownerである`typaxis-syntax`、wire encodingは`typaxis-document-package`のJCS encoderへ一意に置く。これにより、exportとingestionでfield名、enum spelling、JCS member順、integer rangeがずれることを防ぐ。

### 12.2 admissionからtrusted packageまでの状態遷移

```text
Host PACKAGE / optional --package-root
  -> HostMachineInputSession::open
  -> AdmittedPackageBytes
       { session, root-relative uri, immutable raw bytes, byte length, raw sha256 }
  -> HostMachineInputSession::decode_and_bind
       -> StrictDocumentPackageDecoder
  -> SessionBoundDecodedPackage
       { wire DTO, canonical JCS sha256, JsonLocationIndex }
  -> HostMachineInputSession::admit_sources
  -> AdmittedMachineSourceSet
  -> HostMachineInputSession::finish
  -> AdmittedMachinePackage
  -> DocumentPackageParser::parse
  -> MachineParseOutcome::Parsed
       { ValidatedMachinePackage, advisories }
  -> MachinePdfPreflight::check(profile)
  -> MachinePdfPreflightReceipt
  -> resource admission / layout / Display / PDF
```

推奨APIの骨格:

```rust
pub struct HostMachineInputSession { /* non-Clone, private session/root handles */ }
pub struct AdmittedPackageBytes { /* private binding + immutable raw bytes */ }
pub struct WireDocumentPackage { /* caller-constructible untrusted DTO */ }
pub struct DecodedDocumentPackage { /* decoder-issued private binding */ }
pub struct SessionBoundDecodedPackage { /* private session binding */ }
pub struct AdmittedMachineSourceSet { /* actual source bytes + facts */ }
pub struct AdmittedMachinePackage { /* decoded package + exact source set */ }
pub struct ValidatedMachinePackage {
    parsed: Box<ValidatedParsedPackage>,
    provenance: ValidatedMachineProvenance,
}

pub enum MachineParseOutcome {
    Parsed {
        package: ValidatedMachinePackage,
        diagnostics: Vec<AdvisoryDiagnostic>,
    },
    Failed {
        progress: MachineInputProgress,
        failure: ParseFailure,
    },
}

impl HostMachineInputSession {
    pub fn open(
        options: MachineInputHostOptions,
        limits: &ValidatedResourceLimits,
    ) -> Result<(Self, AdmittedPackageBytes), MachineInputError>;

    pub fn decode_and_bind(
        &self,
        raw: &AdmittedPackageBytes,
        decoder: &StrictDocumentPackageDecoder,
        policy: &DocumentPackageDecodePolicy<'_>,
    ) -> Result<SessionBoundDecodedPackage, MachineInputError>;

    pub fn admit_sources(
        &self,
        decoded: &SessionBoundDecodedPackage,
        limits: &ValidatedResourceLimits,
    ) -> Result<AdmittedMachineSourceSet, MachineInputError>;

    pub fn finish(
        self,
        raw: AdmittedPackageBytes,
        decoded: SessionBoundDecodedPackage,
        sources: AdmittedMachineSourceSet,
    ) -> Result<AdmittedMachinePackage, MachineInputError>;
}

impl DocumentPackageParser {
    pub fn parse(
        self,
        input: AdmittedMachinePackage,
        policy: &PackageValidationPolicy<'_>,
    ) -> MachineParseOutcome;
}
```

`parse`は`AdmittedMachinePackage`をconsumeし、大きなsource/text bufferをcloneせず`SourceCatalog`と`TextStore`へmoveする。`ValidatedMachinePackage`は既存`ValidatedParsedPackage`と`ValidatedMachineProvenance`を一緒に所有するwrapperであり、弱いAST typeではない。layoutへ渡す文書本体は`.parsed()`で既存trusted typeをborrowする。provenanceはraw/canonical package identity、profile-independent JSON location index、opaque admission session bindingだけを所有し、ASTの代替にはしない。

portableな`MachineInputFingerprint`はalgorithm ID `typaxis.machine-input-sha256/1`を持ち、次のJCS recordのSHA-256とする。

```json
{
  "algorithm": "typaxis.machine-input-sha256/1",
  "package": {
    "bytes": 12345,
    "canonical_sha256": "<typed-jcs-sha256>",
    "contract": "typaxis.contract/1.1",
    "sha256": "<raw-sha256>",
    "uri": "document-package.json"
  },
  "sources": [
    {
      "bytes": 67890,
      "sha256": "<actual-source-sha256>",
      "source_id": 0,
      "uri": "sources/book.json"
    }
  ]
}
```

host session identity、absolute root、profile ID、configはこのportable fingerprintへ入れない。receiptはportable fingerprintに加えてopaque session identityも照合する。profile/configは後段のcapability/output receiptが別にbindする。

各receiptは発行sessionのopaque identityを持つ。`decode_and_bind`はraw receiptがselfのsession所属であることを検査してから、そのreceiptが所有するexact bytesだけをdecoderへ渡す。`finish`はraw、decoded、source setが同じsession、package hash、source declaration fingerprintへbindされていることを照合する。`Clone`、public field、raw partsからのconstructorは提供しない。bytes/hashが一致しても別sessionのreceiptを混ぜることを拒否する。

parserはinputをconsumeするため、failure variantが最後のvalidated progress tokenを必ず返す。これによりsemantic validation失敗でもraw/decode/source factsをfailed manifestへ記録でき、callerが失敗したDTOからrecordを再構成する必要がない。

failed manifest用には、CLIが任意のrecordを作るのではなく、次の進捗tokenだけをledgerへ渡せるようにする。

```text
NoInput
  -> RawPackageAdmitted
  -> PackageDecoded
  -> SourcesAdmitted
  -> PackageValidated
  -> CapabilityValidated
  -> ResourcesAdmitted
  -> LayoutSelected
```

進捗は単調で、後段tokenが前段factsを内包する。failure pathは最後に発行済みのtokenまでをmanifest/diagnostics publisherへ渡し、未検証値を補完しない。

font/image admissionも同じ方針に合わせる。`AdmittedResourceResolver`は各resourceのbytes/hash/metadata検証完了後にsession-bound `ResourceAdmissionProgressToken`を更新し、failure outcomeが最後のtokenを返す。これはfailed manifest専用で、layout/finalizerは従来どおりcomplete `AdmittedResourceLedger`だけを受理する。

### 12.3 package rootとstable read

`MachineInputHostOptions`はhost-onlyな`package: HostPath`と`package_root: Option<HostPath>`を持つ。wire artifactへabsolute pathを保存しない。

`typaxis-host-admission`のgeneric APIは`OpenedContainedFile { private handle, observed exact length, identity }`と、bounded read後の`StableFileBytesReceipt`を分ける。machine/resource ownerはobserved lengthを自身のper-item/aggregate budgetへreserveしてからread permitを渡す。host ownerはsame handleの実lengthだけを読み、caller-supplied exact lengthや任意`Read`をtrusted inputにしない。

host側にはconfig非依存のfixed `MAX_RESOURCE_ROOTS = 64`と`MAX_HOST_READ_CANDIDATES = 131_072`を置き、`HostCapabilityDescriptor`とcapability JSONの`host_limits`も同じ定数から生成する。前者はproject rootとCLI/configのresource-root entryを合わせた件数へroot identity解決/handle open前に適用し、alias entryも枠を消費した後で別途拒否する。後者はPACKAGE、config、source、および全`resource declaration × admitted resource root`から生じるlogical read candidate attempt数へ適用する。同じtargetへ解決する別declaration/root pairもwork budgetは一件ずつconsumeし、identity ledgerへの保存だけをdeduplicateする。早期に判明するcandidateは各open前に一件ずつconsumeし、package validation後はresource candidateのchecked積を計算して全件分をreserveしてからresource pathを一つでもopenする。max+1 reservationは`I9102`、limit exit 4で拒否する。exact maxは許可する。これによりread/write alias保護そのものをunboundedなメモリ/host syscall増幅経路にしない。

- `--package-root`省略時はPACKAGEのlexical parentをroot、file nameをpackage URIとする。parentが空ならcurrent directoryを使う。
- 明示時はPACKAGEとrootをabsolute lexical pathへ変換してroot-relative pathを導出し、`..`でroot外へ出るものをopen前のusage exit 2で拒否する。
- root-relative pathは`PortablePath`へ変換し、root directory handle相対でPACKAGEをopenする。canonicalize後のabsolute pathをmanifest URIに使わない。
- package root自体のsymlinkをcanonical rootとして解決することは許可するが、root内のPACKAGE/source path componentはsymlinkを許可しない。
- default config lookupとconfig project rootは既存CLIどおりcurrent directory/`--config` parentを使い、package rootへ暗黙変更しない。
- sourceはpackage rootだけから解決する。package rootをfont/image resource rootへ暗黙追加しない。必要ならproducerが`--resource-root`またはconfigで明示する。

Unixのcontained-open実装は共通component walkerを持つ。Linux/Androidではまず`openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS)`を使い、`NOSYS`なら同じcomponent walkerへfallbackする。macOS等はdirectory handleごとの`openat(O_NOFOLLOW)` walkerを使う。中間componentはdirectory、終端はregular fileであることをsame handleの`fstat`で検査する。Windows等で同等のhandle-relative openが未実装なら、package bytesを読む前にstable `UnsupportedContainedOpen`でfail closedする。

PACKAGEとcompanion sourceのread手順は共通化する。

1. no-followでopenし、nonblocking shared lockが利用できるplatformではlockする。
2. same handleからdevice/inode/kind/length/mtime/ctime snapshotを取る。
3. limitとdeclared lengthをallocation/read前に検査する。
4. exact lengthだけをchunk readし、同時にSHA-256を計算する。max+1 probeをしない。
5. same handleを再度statし、snapshot不一致を拒否する。
6. sourceはUTF-8、declared SHA-256、declared byte lengthを照合する。
7. read済みowned bytesだけを以後のphaseへ渡し、pathを再openしない。

M0では既存`typaxis-resource-admission`を全platformでtype-check可能に直す。`HostResourceFile::exact_length`をplatformによって存在しないmethodにせず、`Result<u64, ResourceAdmissionError>`にする。unsupported fallbackは架空のlengthを返さず`UnsupportedContainedOpen`を返す。`PathBuf`、`ConfigResourceRoot`、test helperのimport/cfgを実装domainと一致させる。M1ではこのhost I/O部分を`typaxis-host-admission`へ抽出し、上記macOS component walkerをpackage/source/resource rootへ共通適用して、`paragraph-1`のTrueType fontをmacOSでE2E admitできるようにする。

### 12.4 bounded strict JSON decoder

`ResourceLimits`へ次を追加し、defaults、config/TOML、environment、CLI override、effective JCS、Schema、manifest validationを同時更新する。

| limit/type | default | profile maximum | 適用点 |
|---|---:|---:|---|
| `max_document_package_bytes: u64` | 134,217,728 | JSON safe integer | PACKAGE allocation/read前 |
| `max_json_nesting_depth: u16` | 256 | 256 | typed deserialize前のiterative scan |

`max_document_package_bytes`はreference source用`max_input_bytes`と独立とし、一方から他方を推測しない。companion sourceには既存のper-source/aggregate limitを適用する。`max_json_nesting_depth`はJSON container depthであり、typed AST depthとは別に数える。

CLIはfixed `MAX_MACHINE_DIAGNOSTICS = 256`から`MachineDiagnosticBudget`を一つ作り、decode、syntax、capability、resource/style preflightへ順に貸し出す。各phaseが別々に256件を発行してaggregate上限を超えることを許さない。success時は上限までadvisoryを保持できる。上限到達後に最初のerror/fatalが発生した場合は末尾advisoryを一件evictしてfailureを必ず保持し、省略件数を最後の保持recordのnoteへ記録する。primary failureやfatal terminalをadvisoryより先に捨てない。このcapはsecurity profile定数であり、config/CLIから拡張しない。

decoderは二段に分ける。

1. iterative lexical/structural preflight
   - UTF-8、BOM/raw NUL不許可、root object、single top-level value、trailing token不許可を検査する。
   - `{`と`[`をdepth 1として、max+1 containerへ入る前に拒否する。
   - object frameごとにescape decode済みmember nameのsetを持ち、`"a"`と`"\u0061"`もduplicateとして拒否する。Unicode normalizationやcase foldingは行わない。
   - string/escape/number/literal grammarを検査し、現在のcontainer/fieldとtoken start byte offsetだけをstreaming保持する。
   - token数はraw byte数以下なので別allocation budgetを設けず、全bufferはpackage byte limit内に置く。
2. typed DTO decode
   - pinned `serde`/`serde_json`、`serde_path_to_error`、`serde_stacker`を使い、全objectを`deny_unknown_fields`にする。generic `Value`、`HashMap`、`flatten`は使わない。
   - `serde_json`の固定depth defaultへ依存しない。iterative preflight通過後だけrecursive limitを解除し、`serde_stacker`上でprofile maximum 256以内をdecodeする。
   - recursive collectionは無制限なderive任せにせず、`DeserializeSeed`へlimitsとchecked counterを渡す。Source/TextBuffer/AST node/style/resource count、各text bytes、aggregate text bytesをVec reserve/push前にconsumeする。
   - integer fieldはinteger型へ直接decodeし、fraction/exponent、negative-to-unsigned、range外を拒否する。
   - enum tagとrequired fieldをwire DTOで閉じ、domain constructorを呼ぶ前にshape errorを返す。
   - decode error pathをRFC 6901 JSON Pointerへ変換する。`serde_json`のline/columnはerror時にraw bytesを再走査して0-based byte offsetへ変換し、全token offset tableを保持しない。

decoder-issued `DecodedDocumentPackage`はwire DTO、raw bytes hash、typed canonical JCS hash、`JsonLocationIndex`を持つ。indexはroot member、SourceId、TextBufferIdとmapping index、NodeId、StyleId/source_orderとdeclaration index、MasterIdとselection-rule index、FontFaceId、ImageResourceIdからcanonical Pointerへの対応を持つ。semantic validationやcapability errorはこのindexを使い、host pathを再構成しない。

decoder段階ではcaller IDがdense/canonicalとは限らないため、ID値を長さに使う`Vec`を確保しない。indexはarray ordinalを正本にし、各entityをboundedな`(typed ID, occurrence, ordinal)`列へ保存してID順にstable sortする。lookupはbinary searchし、duplicate ID errorでは二件目のordinalをprimaryにする。semantic validationがdense IDを証明した後だけ、同じordinal tableへbindしたsealed dense lookupを発行してよい。Pointer文字列は複製せず、ordinalとfixed member segmentからdiagnostic発行時にmaterializeする。entry count、sort scratch、Pointer生成bytesは対応するAST/style/text/resource/package budgetをreserve前にconsumeする。

decode diagnosticのprimary位置も固定する。duplicate keyは二つ目のkey token、unknown fieldはそのkey token、type/range errorはvalue token、missing fieldはそれを要求するcontaining object、malformed/truncated JSONは最後に確定したcontainer/tokenを指す。duplicate/unknown keyのPointerは二つ目またはunknown member名を含める。

JCS encoderは`typaxis-document-package`に一つだけ置き、`max_document_package_bytes`をconsumeするbounded `Write`/hash sinkへstreaming出力する。decoderはcanonical bytes全体を二重保持せずSHA-256だけを保存し、`dump-ast`は同じencoderをstdout用sinkへ通す。JCS encoderが受理できない、またはcanonical encodingがlimitを超えるwire DTOをdecoder/export成功にしない。raw hashはwhitespace/member orderを区別し、canonical hashはJCS object member orderの違いを吸収する。

### 12.5 syntax loweringとentry-only closure

`DocumentPackageParser`は次の順でwire DTOをdomain typeへloweringする。

1. exact contract IDとcoordinate unitを検査する。
2. `sources.len() == 1`、`source_id == 0`、declarationとadmitted source setのexact一致を再検査する。
3. actual UTF-8 bytesから`SourceRecord`/`SourceCatalog`を構築する。
4. TextBufferId順に`TextStore`を構築し、mappingの局所shapeを検査する。
5. typed preorderをiterativeに検査してからDocument、StyleSheet、PageMasterSet、ResourceCatalogを構築する。
6. syntax ownerだけがentry-only `ValidatedIncludeGraph`を発行する。source bytesへinclude keyword scanを行わない。
7. private `ValidatedParsedPackage::new_resolved`でSourceSpan、identity bytes、ID closure、style/page/resource semantics、limitsを検査する。
8. success時だけ`ValidatedMachineProvenance`と一緒に返す。

publicな`ParsedPackage -> ValidatedParsedPackage`、`WireDocumentPackage -> ValidatedParsedPackage`、`ValidatedIncludeGraph::machine_entry_only` constructorは追加しない。unit test用fixtureもproduction APIをfeature flagで公開せず、syntax crate内部test moduleのowner constructorを使う。

lowering errorはfield/IDに対応するJSON Pointerを必須にする。SourceSpan/TextMapのbytes照合後にsource locationを特定できるerrorは`source` locationも選べるが、同じdiagnosticにpackage JSONとsourceの二つのprimary locationを持たせない。もう一方はnoteにする。

### 12.6 `paragraph-1` capability gate

`typaxis-machine-profile`は`MachineProfileDescriptor::PARAGRAPH_1`を唯一の正本とする。初期descriptorは次のdomainだけをadvertiseする。

| axis | `typaxis.machine-pdf/paragraph-1` |
|---|---|
| source closure | exactly one source、entry-only |
| block | `paragraph`、`heading` |
| inline | `text`、`anchor`、`reference(format = page)`、`soft_break`、`hard_break` |
| rejected inline | `emphasis`、`strong`、`link`、`footnote_reference`、`reference(text/number)` |
| footnote | definition/referenceとも0件 |
| style property | `font_family`、`font_size`、`line_height`、`page` |
| style selector | `paragraph`、`heading`だけ。unusedでも他block selectorは拒否 |
| page style | `page = auto`のみ。named page requestは拒否 |
| page master | exactly one default master、header/footer/footnote frameはnull、selection ruleは0件 |
| font | TrueType sfnt/TTCの`glyf` outline。textが無ければ0 fontも可 |
| image | declaration、usageとも0件 |
| PDF semantics | text extraction、anchor named destination |
| 明示的な非対応 | link annotation、outline、tagged PDF、heading semantic structure |

headingはM1ではparagraphと同じflow classとして組版でき、levelとanchorをvalidation/fingerprintには保持するが、PDF outline/tagged structureを意味しない。capability JSONはこの制限を`pdf_features`と`unsupported_pdf_features`へ明示する。

profile descriptorとは別に、compiled targetから導出する`HostCapabilityDescriptor`を持つ。macOS/LinuxのM1 releaseではcontained package/resource openとatomic file publishをtrueにする。同等機能が未実装のtargetではprofileを`available = false`として出力し、`build-package`/`check-package`はPACKAGEを読む前に`I9110`、I/O exit 3で拒否する。platform差をprofileの意味へ混ぜず、利用可能性として明示する。

host booleanをCLIへ重複記述しない。machine input owner、resource admission owner、atomic publication ownerがそれぞれcompile-time capability tokenを発行し、CLIがそれらを`HostCapabilityDescriptor`へ合成する。profileの`available`判定と実commandのpreflightは同じtoken集合を使う。

atomic sidecar publisherは利用できるがcontained openだけが不足するtargetでは、requested diagnostics/failed manifestへ`I9110`を記録する。atomic publisher自体が利用できないtargetではpublication context構築が先にI/O exit 3で失敗し、全targetを変更せずstderrだけへ報告する。

preflightは`ValidatedMachinePackage`のtyped Document preorderをiterativeに走査し、unsupported itemをNodeId昇順に収集する。style/page/resourceのglobal errorはDocument traversal後に、`style rule source_order -> master ID -> resource ID`の順で収集する。同じpackageではplatform、HashMap insertion、thread completionに依存せず同じdiagnostic列になる。

unsupported件数が残りdiagnostic budgetを超えた場合も全ASTをboundedに走査してgate失敗を確定し、budget内の先頭件だけをmaterializeする。最後のrecordのnoteへ「追加N件を省略」と記録し、別のtruncation recordで上限を超えない。fatalはその場で終端し、後続を収集しないという既存diagnostic規則を維持する。

成功receiptは少なくとも次をbindする。

```rust
pub struct MachinePdfPreflightReceipt {
    profile: MachinePdfProfileId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    package_input: MachineInputFingerprint,
}
```

machine layout entrypointは`ValidatedParsedPackage`だけを受けず、`&ValidatedMachinePackage`とこのreceiptを同時に要求する。内部で`.parsed()`と`.provenance()`のfingerprintを再照合し、binding不一致はinternal invariant errorにする。preflight通過後にreference-only layout/displayが「unsupported domain」を返した場合もuser input errorへ戻さず、descriptorと実装がずれたinternal errorとして扱う。

`typaxis capabilities --format json`は同じdescriptorから生成する。JSON encoder用の別feature listを手書きしない。各advertised featureには単独fixtureと、全advertised featureを一つに組み合わせたE2E fixtureを必須にする。

capability artifactにはmachine packageをdecodeする前にproducerが必要とする`max_document_package_bytes`と`max_json_nesting_depth`のbuilt-in default/hard profile maximumも、同じ`ResourceLimits` descriptorから出す。既存のsource、AST、text、resource、layout、PDF limitはpackage config Schemaの1.1契約を正本とし、capability artifactへ値を重複させない。capability側の値はper-job EffectiveConfigではないため、CLI/config override後の実効値を表すとは記載しない。実効configはmanifestの`config_sha256`へbindする。

### 12.7 CLIとphase orchestration

CLI typeは既存`BuildOptions`を流用してfieldをoptionalに増やさず、commandごとに分ける。

```rust
pub struct BuildPackageOptions {
    package: PathBuf,
    package_root: Option<PathBuf>,
    profile: Option<String>,
    output: OsString,
    trace: Option<PathBuf>,
    manifest: Option<PathBuf>,
    diagnostics: Option<PathBuf>,
    force: bool,
    common: CommonOptions,
}

pub struct CheckPackageOptions {
    package: PathBuf,
    package_root: Option<PathBuf>,
    profile: Option<String>,
    diagnostics: Option<PathBuf>,
    common: CommonOptions,
}
```

command dispatchは`Build`、`BuildPackage`、`Check`、`CheckPackage`を明示variantにする。source commandとpackage commandのhelperは、config optionなど同一のtoken grammarだけを共有し、input loaderを共有しない。

`check-package`のCLI grammarが共有するのは`--config`、`--resource-root`、`--profile`、`--package-root`、`--emit-diagnostics`、limit overrideだけとする。layout/outputにしか意味を持たない`--strict`、`--no-compress`、`--trace`、`--force`はusage errorにし、受理して無視しない。config file内の同fieldは通常どおりEffectiveConfigへ含める。

`run_build_package`は次のphaseを一方向に実行する。

```text
CLI/config/target validation
 -> output/manifest/diagnostics publication contexts
 -> compiled host capability validation
 -> package root + raw package admission
 -> strict JSON decode
 -> companion source admission
 -> syntax lowering / trusted package
 -> safe declared resource candidate registration + read/write alias gate
 -> paragraph-1 capability preflight
 -> font/image resource admission
 -> style/font coverage preflight
 -> paragraph flow / pagination / Display / PDF
 -> terminal publication
```

M1 ownerはこれらのphaseをsingle-threadedに実行してよい。将来source/resource decodeを並列化しても、ID順にjoinしてからreceipt/diagnosticを発行し、worker completion順をallocation、primary error、manifest orderへ使わない。

`check-package`はraw package admissionからstyle/font coverage preflightまでを同じfunctionで実行し、layout、trace、PDF、manifestを作らない。したがって成功はJSON shapeだけでなく、trusted source、semantic package、capability、resource metadata、全text-producing siteのcomputed style/font family解決までを保証する。pagination convergence、final line layout、PDF serialization成功までは保証しない。この境界をhelpとproducer guideへ明記する。

style/font coverage preflightは全text-producing siteとgenerated site registryをtyped preorderで走査し、`cascade_style`、family解決、font instance tableとのbindingを検査する。glyph coverageを「font familyが解決した」ことと混同せず、実際のshapingでしか判定できないmissing glyphはbuild phase errorになり得ることをdiagnostic contractに記載する。

`capabilities`はconfig、filesystem、ambient localeを読まず、compiled engine/profile/host descriptorだけからcanonical JSONをstdoutへ出す。`--format json`を必須とし、unknown formatをusage exit 2で拒否する。`build-package`/`check-package`のunknown profileもusage exit 2とする。

`BuildExecutionContext`はdiagnostics targetを含むtarget setへ拡張する。output、trace、manifest、diagnosticsのcanonical parent+leafと既存file identityを構築時、各temp write前、各publish直前に再検査する。`check-package`はPDF outputを持たない`DiagnosticsExecutionContext`を使い、build contextへdummy outputを渡さない。

write target同士だけでなく、PACKAGE、source、config、font/imageとのaliasも拒否する。CLI parse時に判明するPACKAGE/config、source openerが解決を試みるcandidate、trusted package発行後に判明する全safe font/image URIと全resource rootの組から導出したcandidateのcanonical parent+leafを、各open前かつcapability gate前にsealed `HostReadIdentityLedger`へ登録する。unsupported resourceはbytesをopenしないがcandidateは登録するため、failure diagnostics/manifestがその既存fileを上書きしたりmissing candidate pathを新規作成したりしない。open成功時はsame-handle identityを追加する。content validation成否にかかわらずfailure outcomeも最後のread-ledger tokenを返す。各publish直前にread/writeのlogical path targetとidentityを再照合し、`--force`でもinputを上書き・欠落input pathを作成しない。host path/identityはexecution contextだけに保持してmanifestへserializeしない。

### 12.8 build manifestとcontract version

manifestとdiagnosticsのwire shapeを変更するため、同じ`typaxis.contract/1.0` IDの意味を上書きしない。M1 change setでcurrent output contractを`typaxis.contract/1.1`へ進め、Schema `$id`、Rust constant、全encoder、minimal/conformance/invalid fixture、validator、docsを同時更新する。変更前のSchema一式を`schemas/1.0/`へfrozen copyし、通常の`schemas/*.schema.json`はcurrent 1.1を指す。validatorは両directoryを別registryで検証する。

M1のDocumentPackage shape自体は1.0と1.1で同じに保つ。machine decoderはknown contractとして1.0と1.1を明示的に受理し、unknown IDを拒否する。`dump-ast`は1.1を出力する。1.0 inputを1.1へ黙って書き換えず、typed canonical hashはinputに記載されたcontract IDを含めて計算する。

既存reference buildを移行可能にするため、raw `typaxis.toml` loaderもknown inputとして1.0と1.1を受理し、merged `EffectiveConfig`はcurrent 1.1へ正規化する。1.0 configでは新しい二limitをbuilt-in defaultとして補ってからCLI/environment overrideを適用し、明示された1.1値と同じ順序でvalidationする。config hashは正規化後の1.1 JCSから計算する。build manifest、trace、diagnostics等のgenerated artifactは1.1だけを出力し、1.0 consumerが同じIDで新shapeを誤受理しないようにする。

`BuildManifest`へ次を追加する。

```rust
pub enum BuildInputProfile {
    ReferenceSource1,
    MachinePdfParagraph1,
}

pub struct PackageInputRecord {
    uri: PortablePath,
    bytes: u64,
    sha256: [u8; 32],
    contract: Option<DocumentPackageContractId>,
    canonical_sha256: Option<[u8; 32]>,
}
```

wire fieldは`input_profile`と`package_input`で、次のconditional ruleを持つ。

| mode/status | `input_profile` | `package_input` |
|---|---|---|
| reference built/failed | `typaxis.reference-source/1` | 常にnull |
| machine built | `typaxis.machine-pdf/paragraph-1` | non-null、`contract`/`canonical_sha256`もnon-null |
| machine failed before raw admission | machine profile | null |
| machine failed after raw admission | machine profile | raw factsはnon-null、decode前なら`contract`/`canonical_sha256`はnull |
| machine failed after decode | machine profile | 全field non-null |

`inputs`はcompanion sourceだけをSourceId順に保持し、package JSONを重複して入れない。package rootやabsolute PACKAGE pathは保存しない。

`BuildOutputCommitContext`作成時にresolved `BuildInputProfile`をbindし、後からmanifest callerが別profileへ差し替えられないようにする。machine built preflightは`ValidatedMachineProvenance`と`MachinePdfPreflightReceipt`を要求し、raw/canonical package identity、profile、Document/Style fingerprint、source ledger、pagination/PDF receiptを同時照合する。

failed ledgerのmethodはsealed progress tokenだけを受ける。

```rust
ledger.admit_raw_machine_package(raw.token())?;
ledger.admit_decoded_machine_package(decoded.token())?;
ledger.admit_validated_package(machine.provenance_token())?;
ledger.admit_resource_progress(resource_progress.token())?;
ledger.admit_resources(admitted.token())?;
```

complete resource ledgerがある場合は同じresource progressを置換・完成させ、manifestへ重複recordを作らない。record field値を引数で直接渡すAPIは作らない。後段tokenをadmitするときは、ledger内の前段bindingとexact一致するか検査する。

### 12.9 structured diagnosticsとpublication

1.1 diagnostics wireは旧nullable field群を`location` tagged unionへ置き換える。

```rust
pub enum DiagnosticLocation {
    PackageJson {
        uri: PortablePath,
        json_pointer: JsonPointer,
        byte_offset: Option<u64>,
    },
    Source {
        source_span: Option<SourceSpan>,
        text_span: Option<TextSpan>,
        node_id: Option<NodeId>,
    },
}
```

`Source` variantは3 fieldの少なくとも1件を必須にする。global config、I/O、publication errorは`location = null`を許可する。JSON Pointer rootは空文字列、member escapeは`~0`/`~1`、array indexはleading zeroなしdecimalとする。byte offsetはraw PACKAGE bytesの0-based offsetで、semantic error等で正確に求められない場合だけnullにする。

canonical diagnosticsの`message`/`notes`にもabsolute HostPath、raw OS error文字列、source/package本文snippetを入れない。package/source/resourceはadmit済みlogical URIで表し、platform固有pathと詳細なOS errorはstderrだけへ出す。同じlogical failureのsidecar bytesがcheckout rootによって変わらないことをreproducibility testへ含める。

`Diagnostic`へlocation/notesのvalidated builderを追加し、wire encoderがprivate fieldを直接読む。CLIの`Failure { kind, message }`からcodeを文字列解析する方式はmachine pathで使わない。各phaseはtyped `Diagnostic`/`ParseFailure`を返し、stderr formatterとJCS sidecar encoderが同じ値を読む。exit kindは最高severityではなく、typed failure categoryから既存の1/3/4/5へ決定する。

現在logical IDを失うunit-like errorはmachine pathへそのまま流さない。resource admissionはFontFaceId/ImageResourceIdまたはlogical URI、style resolutionはNodeId/StyleId/property、shaping/layoutはNodeIdと可能ならTextSpan、page validationはMasterId/rule indexを持つtyped error subjectを返す。diagnostic mapperはsubjectを`JsonLocationIndex`またはsource/text locationへ引き、`Debug`文字列からIDやpathを逆解析しない。

`--emit-diagnostics`指定時は成功でも`diagnostics: []`またはadvisory列を出す。processing failureではerror/fatalを含むsidecarを出し、PDFを出さない。sidecar自身のI/O failureはexit 3とし、stderrへprimary/secondary failureを両方記録する。

複数fileを一つのrenameでcommitできないため、「全targetの同時atomic」を約束しない。各fileは個別にtemp write/fsync/atomic publishし、terminal publisherはvisible orderを固定する。

- processing failure: diagnostics、failed manifestの順。PDFはpublishしない。
- file build success: trace、PDF、diagnostics、built manifestの順。built manifestを最後のterminal recordにする。
- stdout build success: PDF stream完了後、diagnostics、built manifestの順。stdout部分writeはrollback不能として既存receipt規則を維持する。
- `check-package`: diagnosticsだけをatomic publishする。

built pathの全temp bytesとmanifest preflightは最初のpublish前に完成させる。途中のpublish failureは、それ以前にvisibleとなったartifactと未publish artifactをtyped partial-publication errorに保持する。既存fileを「失敗時に元へ戻す」とは表現しない。

processing failureではdiagnostics publishが失敗してもfailed manifest publishを一度だけ試行し、両方の結果をcombined errorへ保持する。build successではPDF後のdiagnostics publishが失敗した場合、terminal built manifestをpublishせずpartial publicationとして返す。

traceまたはPDF sinkがbuilt planのcommit中に失敗した場合は、publisher-issued I/O diagnosticを追加したfailure diagnosticsと、同じprogressからpreflight済みのfailed manifestをpublishする。stdout partial writeは`output = null`のまま、partial streamがrollback不能だったことをstderr/diagnostic noteへ残す。PDF fileのatomic publish後にdirectory syncだけが失敗した場合はvisible PDF receiptを保持する既存durability-uncertain variantとし、存在するPDFを「生成されなかった」failed recordへ書き換えない。

### 12.10 initial diagnostic code割当

M1で追加するprimary codeを次に固定する。詳細なdomain errorを全て`P1000`や`L5000`へ潰さない。

| code | category | 意味 |
|---|---|---|
| `P1100` | input | PACKAGEのUTF-8/BOM/NUL/root/trailing token違反 |
| `P1101` | input | JSON grammarまたはduplicate key |
| `P1102` | input | unknown/missing field、type、integer range、enum違反 |
| `P1103` | input | unsupported contract/coordinate unit |
| `P1110` | input | machine source profile違反（count、SourceId、order） |
| `P1111` | input | unsafe/non-contained companion source URI/path |
| `P1112` | input | declared source length/hashとactual bytesの不一致 |
| `L5100` | input | unsupported block/inline/reference/footnote capability |
| `L5101` | input | unsupported style/page-master capability |
| `R7100` | input | unsupported resource declaration/format capability |
| `I9100` | limit | `max_document_package_bytes`超過 |
| `I9101` | limit | `max_json_nesting_depth`超過 |
| `I9102` | limit | fixed host resource-root/read-candidate上限超過 |
| `I9110` | I/O | compiled targetでrequired host capabilityが利用不可 |
| `I9111` | I/O | package root/PACKAGEのcontained open失敗 |
| `I9112` | I/O | companion sourceのopen/read/lock system failure |
| `I9113` | I/O | package/sourceがstable read中に変化した |
| `I9190` | internal | capability receiptと下流domainの不一致 |

source/text/style既存validatorのerrorは対応する既存prefixへmapする。mapping tableを`typaxis-syntax`の一箇所に置き、`Debug`文字列を公開message/codeへ使わない。公開codeの追加・意味変更にはdiagnostics Schema fixtureとCLI E2Eを必須にする。

## 13. M2以降へ拡張するlayout/display設計

### 13.1 canonical flow registry

現在の`CanonicalFlowIrBuilder`にはlist/table/figure用boundaryがあるが、paragraph registryを必須とし、callerがboundaryをpushする。general pipelineでは、caller-selected順をtrustせず、validated content receiptを登録してbuilder自身がtyped Document preorderを走査する形へ一般化する。

```rust
pub enum ValidatedFlowContent {
    Paragraph(ValidatedParagraphBreak),
    ListItem(ValidatedListItemLayout),
    TableRow(ValidatedTableRowLayout),
    Figure(ValidatedFigureLayout),
    PageBreak(ValidatedPageBreak),
}

pub struct ValidatedFlowContentRegistry { /* package/epoch-bound */ }
pub struct ProductionFlowIrBuilder<'a> { /* package + complete registry */ }
```

registryはNodeIdごとにexpected kind、owner-local boundary count、child flow ID、LayoutEpochを照合する。`finish`はDocument indexからboundaryをcanonical順で発行し、missing、extra、wrong kind、wrong epochを拒否する。worker completion順やcaller insertion順はFlow ordinalへ影響させない。

subflowは本文cursorへ平坦化しない。少なくとも次を別`FlowId`とterminalで登録する。

- document body
- list item child blocks
- figure caption
- table cell
- footnote definition
- 将来のheader/footer、column、float

`ValidatedFlowRegistry`は全FlowIdをcanonical owner順にdense allocationし、各subflowのowner、parent relation、terminal、package/epoch fingerprintを持つ。pagination traceとselected stateはbodyだけでなくregistry全体をbindする。

### 13.2 M2: list、page break、figure、link

M2は一機能ずつvertical sliceで実装し、同じprofile IDの意味を後から増やさない。追加profileはたとえば`typaxis.machine-pdf/basic-document-1`とし、`paragraph-1`はfrozen subsetとして残す。

list:

- markerをcaller stringではなく`ordered/start/item_index`からchecked生成し、`GeneratedBufferKey`へ登録する。
- markerとitemの最初のpainted lineを同一fragment receiptへbindし、markerだけをpage末へ残さない。
- item child blocksは独立subflowとし、nested listも同じprogress規則を再帰ではなくflow stackで処理する。
- ordered marker overflow、empty painted item、exact fragment limitをunit/E2Eで検査する。

page break:

- `PageBreak`はzero-size contentではなくtyped forced-boundaryとする。
- empty frame先頭でもboundaryを一度consumeして次cursorへ進め、同じcursorを`More`で返さない。
- 連続page breakとdocument末尾page breakのblank page policyをprofileへ固定し、trace/PDF page countで検査する。

figure/PNG:

- M2ではinline floatを実装せず、non-floating block placementだけに限定する。
- figureはcomputed `width`を必須とし、heightはadmitted PNG pixel aspect ratioからfixed-point checked roundingで導出する。暗黙のpixel-to-point DPIを導入しない。
- image placement、caption subflow、alt text、ImageResourceIdを一つの`ValidatedFigureLayout`へbindする。
- Display painterはexact placementから`DrawImage`を一件発行する。usage collector、admitted ledger、late finalizer、PDF XObjectのmissing/extra/wrong-IDを閉じる。
- captionが同一pageに収まらない場合のkeep policyをtyped styleで選び、未実装policyはpreflightで拒否する。

link:

- link child rangeをparagraph itemization時にlogical cluster rangeへbindし、selected lineごとにvisual rectangleのunionを作る。
- internal linkはpackage anchorからselected named destinationへ、external linkはvalidation済み`SafeUri`へ結ぶ。
- rectangleはpage bounds内へvalidateし、空childrenまたはpainted clusterを持たないlinkはM2 profileで拒否する。
- Display/PDF closureはlinkごとに少なくとも1 annotationを要求し、missing、extra、wrong page、wrong targetをnegative testにする。

M2でstyle registryへspacing、indent、alignment、width、keepを追加するときは、property name、wire tagged value、initial/inherit、cascade、layout consumer、capability entry、fixtureを同時に追加する。unknown propertyをlayout codeで文字列比較しない。

### 13.3 M3: tableとfootnote

table layoutはcolumn resolution、cell subflow layout、row fragmentationの三段に分ける。

1. available inline sizeからfixed columnsを引き、remainingをfraction weightへcanonical roundingする。最後のcolumnだけへrounding residualを割り当てる。
2. grid validatorが発行したcell origin/span receiptを使い、各cellへ独立subflow/frameを作る。
3. row fragmentは全active cellの次break candidateから共通block sizeを選び、rowspan continuationを1次元stateで運ぶ。

header row repeatはpage先頭のcloneではなく、original header subflowとselected repetition indexをbindしたfragment receiptとして表す。empty pageでも1行も進められないrowはoversize policyへ一度だけ遷移し、同じcandidateを再評価し続けない。split禁止cell、border collapsing、vertical alignment等を未実装のまま黙ってdefaultにせず、profileで明示拒否する。

footnoteはbody flowと別の`FootnoteFlowId`を持つ。各page passで次を行う。

1. body candidateから初出FootnoteIdをlogical orderで収集する。
2. definition subflowをFootnoteId順ではなくfirst-reference順にmaterializeする。
3. reserved footnote heightを更新してbodyをreflowする。
4. body fingerprint、ordered footnote set、各footnote continuation、reservationが一致したらpage内convergedとする。
5. `max_footnote_reflows_per_page`でmax+1 evaluation前に停止する。

footnote continuationをbody cursorへ混ぜず、next pageへ専用carry receiptで渡す。同一definitionの重複paint、未参照definitionのpaint、referenceのあるdefinition欠落をselected-state closureで拒否する。

### 13.4 M4: modelとpublication contractのversioning

math/vector/semantic container/tagged PDFは既存nodeやPNGへloweringして追加しない。M4開始前に少なくとも次のADRを採択する。

- semantic container kindとchild ownership
- inline/display math source、speech/ActualText、vector paintの三者binding
- safe vector IRまたはsafe SVG subsetと禁止機能
- document metadata、BCP 47 language、outline hierarchy
- semantic nodeからPDF structure tree/marked contentへのreceipt
- JPEG、OTF/CFFを含むmedia/font profile別PDF embedding plan

このADRによりDocumentPackage wire shapeが変わる場合は新contract IDを発行する。既存profileが新nodeを受理したことにせず、new profileだけがnew contract/node setをadvertiseする。

### 13.5 capability互換性

profile IDはimmutableなclosed contractである。

- feature追加、既存拒否の受理、既定layout policy変更はnew profile IDを作る。
- `--profile`省略時の`default_profile`もCLI contractの一部とし、contract 1.1中は`paragraph-1`から変更しない。
- diagnostic message改善は同じprofileでよいが、code/location/primary error順の意味変更はcontract reviewを行う。
- bug fixで本来advertise済みのfeatureを正しくする場合はIDを維持し、regression fixtureを追加する。
- security上advertised featureを停止する場合はengine versionでfail closedし、capability outputから削除するだけで旧IDを別意味へ再利用しない。
- manifestはresolved profile IDを必須にし、producer requestとpreflight receiptの一致を検査する。

## 14. 実装sliceとfile map

M0/M1は次の順で実装する。各sliceはworkspaceをcompile可能に保ち、public commandは最後のE2E closureが揃うまでhelp/capabilityへ公開しない。

### Slice 0: build baseline

- `typaxis-resource-admission`のplatform `cfg`とfallible exact-length APIを修正する。
- macOSでlocked build/check/test/clippy、blank PDF smoke testを通す。
- Slice 0時点ではresourceありfixtureがmacOSでcompileし、runtimeにstable unsupported errorを返す。
- support matrixへcompile、atomic publish、contained resource openを別列で記載する。

### Slice 1: wire contractとlimits

- `typaxis-document-package`を追加する。
- wire DTO、iterative JSON preflight、typed decode、JCS encoder、location indexを実装する。
- 2 limitとfixed diagnostic cap、1.1 contract/schema/fixturesを追加する。
- 全DocumentPackage fixtureをSchema validatorとRust decoderの両方へ通し、accept/reject、conformance `rule_id`、公開diagnostic codeを別namespaceとして検査する。
- `typaxis-syntax`へ全current 1.1 domain variantのexhaustive domain-to-wire変換を追加し、`dump-ast`をshared DTO/encoderへ移す。machine profileのsubset判定をserializerへ混ぜない。
- `dump-ast`はcount/hash sinkで全encodingをpreflightしてからstdoutへ二回目をstreamし、encoder/limit errorでpartial JSONを出さない。
- 既存goldenとの意図差分をreviewし、supported round-trip fixtureを固定する。

### Slice 2: host admission

- `typaxis-host-admission`を追加し、contained root/open、stable read、read identity ledgerを既存resource crateから抽出する。
- `typaxis-machine-input`を追加する。
- package root handle、PACKAGE/source stable read、single-source admission、session receiptを実装する。
- Linux/Android/macOSのpackage/source contained-openを実装し、同じmacOS walkerを既存font/image resource admissionへ適用する。
- unsupported platform fallbackと、macOSのfont付きparagraph E2Eをtestする。
- host root/read candidate attemptのexact fixed max/max+1と、重複targetでwork budgetは別consume・identity ledgerはdeduplicateされることをtestする。
- raw/decode/source各progress tokenを発行する。

### Slice 3: syntax trust boundary

- `DocumentPackageParser`、wire-to-domain lowering、entry-only graph、machine parse outcomeを追加する。
- JSON Pointer付きsemantic diagnostic mappingを実装する。
- caller-authored DTO/ParsedPackageをpromotionできないcompile-fail testを追加する。
- 1.0/1.1 known contractとunknown contractをtestする。

### Slice 4: capability profile

- `typaxis-machine-profile`と`paragraph-1` descriptorを追加する。
- deterministic preflight、receipt、capabilities JCSを実装する。
- `schemas/machine-capabilities.schema.json`とpositive/invalid fixtureを追加し、descriptor由来outputを検証する。
- machine layout wrapperへreceipt bindingを必須にする。
- 全accepted/rejected enum variantをexhaustive unit testで覆う。

### Slice 5: CLI、manifest、diagnostics

- `build-package`、`check-package`、`capabilities`を追加する。
- build/check common preparationを一つのfunctionへまとめる。
- manifest input profile/package identityとsealed progress ledgerを追加する。
- resource admission failure outcomeとpartial progress tokenをmanifest ledgerへ接続する。
- diagnostics location union、sidecar encoder、target alias/terminal publicationを追加する。
- help、producer guide、sample layoutを更新する。

### Slice 6: E2E closureと公開

- positive/negative/tamper/limit/reproducibility/round-trip fixtureを追加する。
- `capabilities`に列挙したfeatureのcombined packageをPDFまで通す。
- Schema validator、Rust全target、CLI smoke/differentialを通す。
- support matrixを実測結果へ更新してからcommand/profileを公開する。

主なfile map:

| path | 変更 |
|---|---|
| `workspace/Cargo.toml`、`workspace/Cargo.lock` | 4 crate member、pinned dependencies、locked resolution |
| `workspace/crates/typaxis-core/src/lib.rs` | limits、`JsonPointer`、contract/profile/input fingerprint newtype |
| `workspace/crates/typaxis-host-admission/` | root handle、contained open、stable read、read identity ledger |
| `workspace/crates/typaxis-document-package/` | wire DTO、decoder、JCS、location index |
| `workspace/crates/typaxis-machine-input/` | host root/session/source admission receipt |
| `workspace/crates/typaxis-syntax/src/lib.rs` | machine parser/lowering/provenance |
| `workspace/crates/typaxis-machine-profile/` | descriptor、preflight、capabilities artifact |
| `workspace/crates/typaxis-diagnostics/src/lib.rs` | location union、builders、encoding input |
| `workspace/crates/typaxis-manifest/src/lib.rs` | input profile/package record、progress ledger |
| `workspace/crates/typaxis-cli/src/cli.rs` | 3 commandとoptions |
| `workspace/crates/typaxis-cli/src/main.rs` | terminal machine orchestration |
| `workspace/crates/typaxis-cli/src/pipeline.rs` | common prepare、receipt-required machine wrapper |
| `workspace/crates/typaxis-cli/src/artifacts.rs` | shared DocumentPackage encoder利用、capability/diagnostic sidecar |
| `schemas/` | 1.1 schema set、1.0 frozen set、invalid/cross rules |
| `samples/machine-package/` | runnable positive/negative bundle |
| `adr/ADR-0027-machine-document-package-ingestion.md` | 本章のtrust/compatibility decision |
| `docs/02-workspace-boundaries.md` | host-admission/resource/machine/syntax dependency edge |
| `docs/26-machine-input-cli.md` | producer向けnormative guide |

## 15. M0/M1 test matrix

### 15.1 decoder/admission/validation

| case | primary phase/code | PDF | diagnostics | manifest facts |
|---|---|---:|---|---|
| 1.1 blank single source | success | 1 blank page | empty/advisory | raw+canonical package、source |
| 1.0 blank compatibility input | success | 1 blank page | empty/advisory | input contract 1.0を保持 |
| paragraph+heading+page reference | success | text PDF | empty/advisory | profile `paragraph-1` |
| BOM/raw NUL/trailing token | `P1100` | none | package offset | raw packageのみ |
| malformed JSON/duplicate escaped key | `P1101` | none | Pointer/offset | raw packageのみ |
| unknown/missing field、float integer | `P1102` | none | Pointer/offset | decoded未成立 |
| unknown contract | `P1103` | none | `/contract` | raw packageのみ |
| package bytes exact max/max+1 | success / `I9100` | success / none | deterministic | rawはlimit内だけ |
| JSON depth exact max/max+1 | success / `I9101` | success / none | deepest Pointer | raw package |
| two sources/nonzero entry ID | `P1110` | none | `/sources` | decoded package |
| source symlink/root escape | `P1111` | none | source URI Pointer | decoded package |
| PACKAGE outside explicit root | usage exit 2 | none | context成立前なら変更なし | manifest変更なし |
| PACKAGE symlink/unsafe contained open | `I9111` / exit 3 | none | globalまたはsafe package URI | package未admit |
| source length/hash mismatch | `P1112` | none | source declaration Pointer | decoded package、source未admit |
| package/source mutation during read | `I9113` | none | global/package location | stable factsまで |
| invalid identity TextMap bytes | `T2xxx` | none | source or JSON Pointer | source factsあり |
| list/link/emphasis/footnote | `L5100` | none | NodeId Pointer順 | validated package |
| named page/header frame | `L5101` | none | style/master Pointer | validated package |
| image declaration | `R7100` | none | ImageResourceId Pointer | validated package、image未read |
| unavailable compiled host capability | `I9110` / exit 3 | none | global location | package未read |
| unknown `--profile` | usage exit 2 | none | context成立前なら変更なし | manifest変更なし |
| 1.2 blank current input under `paragraph-1` | success | 1 blank page | empty/advisory | input contract 1.2、default profileを保持 |

### 15.2 closure、publication、reproducibility

- `check-package`成功fixtureは同じconfig/resourcesの`build-package`でcapability/resource preflightを再通過する。
- arbitrary JSON bytes、Unicode escape key、深いcontainerのproperty testでpanic、stack overflow、duplicate-key見逃しがないことを確認する。長時間fuzz gateはM5で追加する。
- raw/decoded/source receiptを別session間で入れ替え、bytes/hashが同じでもbinding mismatchになることを確認する。
- diagnostic件数のexact `MAX_MACHINE_DIAGNOSTICS`とmax+1で、aggregate上限、advisory eviction、primary/fatal保持、省略noteを検査する。
- raw config 1.0/1.1が同じ1.1 EffectiveConfig/hashへ正規化され、unknown config contractが拒否されることを確認する。
- unsupported featureではresource opener、layout temp、PDF tempが呼ばれていないことをspy/test ownerで確認する。
- output/trace/manifest/diagnosticsの全pairと、各write target対PACKAGE/source/config/used resourceについてlexical alias、symlink alias、hard-link alias、publish直前raceをtestする。
- processing failureではPDFが存在せず、diagnosticsとfailed manifestのstatus/factsが一致する。
- diagnostics publish failure、PDF publish failure、manifest publish failureでvisible artifact集合をtyped errorどおり検査する。
- 同じpackage/root/config/resource/profileを二つの独立checkoutでbuildし、PDF、trace、manifest、diagnosticsのbytesをexact比較する。
- whitespace/member orderだけが違う二packageでraw hashが異なりcanonical hashとDocumentFingerprintが一致する。
- semantic fieldが違う二packageでcanonical hashまたはDocumentFingerprintが必ず異なる。
- supported reference sourceの`dump-ast -> build-package`でcanonical hash再encodeとDocumentFingerprintが一致する。
- capability JSONの各advertised itemを削除/追加したmutant descriptor testにより、fixtureまたはpreflight coverageが失敗することを確認する。
- current sourceからbuildしたCLIのversion、Git revision、binary SHA-256をintegration evidenceへ記録し、既存target binaryを使わない。

### 15.3 validation commands

M0/M1のmerge gateは少なくとも次を含む。

```text
cargo fmt --manifest-path workspace/Cargo.toml --all -- --check
cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked
cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings
python3 schemas/validate.py
python3 tools/verify_reproducibility.py --repository . --revision HEAD
```

加えて、M1 positive PDFを`verify_pdf_differential.py`へ渡し、page count、raster、text extractionを検査する。required external renderer/extractorが無い環境をsuccess skipにしない。

## 16. M0/M1 definition of done

M0/M1は次をすべて満たした一つの公開単位として完了する。

1. documented macOS/Linux targetでcurrent sourceのlocked build/check/test/clippyとfont付き`paragraph-1` E2Eが成功する。
2. `build-package`、`check-package`、`capabilities`のhelpとactual parserが一致する。
3. package/source admission receiptを経由しないmachine promotion pathがcompile-failになる。
4. 1 sourceのactual bytesを使ってSourceSpan/identity TextMapが再検証される。
5. `paragraph-1` descriptor、preflight、layout wrapper、capability JSON、manifest profileが同じID/fingerprintへbindされる。
6. unsupported node/resource/styleはresource read/layout開始前にstable diagnosticで拒否される。
7. built manifestがraw/canonical package、source/resource、selected layout、PDF outputをbindする。
8. requested diagnostics sidecarがsuccess/failureのtyped diagnostic列をcanonical JCSで出す。
9. target alias、stable read、limits、tamper、partial publicationがfail closedでtestされる。
10. two-build/independent-checkout reproducibility、round trip、renderer/extractor gateが成功する。
11. Schema、ADR、CLI guide、samples、support matrixが実装と一致する。
12. M2以降のfeatureをcapability artifactで誤ってadvertiseしない。

このDoDを満たす前は、decoderだけ、CLI commandだけ、またはblank fixtureだけが完成してもmachine input対応済みとしない。
