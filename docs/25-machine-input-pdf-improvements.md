# Machine input PDF統合の不足機能・文書改善計画

## 1. 文書情報

- 状態: 調査済み改善提案（本文で「実装済み」と明記した項目を除き未実装）
- 調査基準: Typaxis commit `6d9be4e20fb02901b5ff4f1bf4ef36643f4fd9e8`
- 調査日: 2026-08-25
- 検証host: `aarch64-apple-darwin`、Rust `1.97.1`
- 対象: CLI、canonical `DocumentPackage`、source/include trust、layout、Display、PDF、manifest、diagnostic、関連文書
- 主な利用者: Typaxisへ文書を機械生成して渡すproducer。特に、VMBのように自前のASTを持つ上流システム

本書は、現行実装の到達範囲を調査した結果と、machine-readableな文書入力からPDFを生成できるようにするための改善要求をまとめる。既存の設計契約を実装済みと読み替える文書ではなく、未実装箇所を明示するためのstatus/gap文書である。

## 2. 結論

現行Typaxisは、machine inputからPDFを生成できない。

`schemas/document-package.schema.json`、portable validator、`dump-ast --format json`は存在するが、これらはCLI ingestion APIではない。現在の`typaxis build INPUT`はINPUTをUTF-8のreference TSFとして読み、sealed `ReferenceParser`へ渡す。JSONの`DocumentPackage`を復号して`ValidatedParsedPackage`へ昇格するcommand、decoder、source admission、trusted receiptは存在しない。

加えて、調査対象commitはmacOS上のclean locked build/checkが`typaxis-resource-admission`のplatform `cfg`不整合でコンパイル失敗する。machine input実装以前に、documented hostで現在のsourceからCLI binaryを作れるbaselineを復旧する必要がある。source tree内の既存`workspace/target/debug/typaxis`が動いても、対象commitの実装証拠にはならない。

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
- 1 source profileでは`SourceId = 0`を要求し、machine package ownerがentry-only closureを発行する。arbitrary source bytesへreference TSFのinclude keyword scanを適用しない。
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
- `package_input`はpackage-root-relative URI、raw byte length、raw SHA-256、typed canonical JCS SHA-256を持つ。
- source modeは`input_profile = "typaxis.reference-source/1"`かつ`package_input = null`、machine modeはversioned machine profileとnon-null recordを要求する。
- failed manifestも、admit済み段階までのpackage/source factsだけをsealed ownerから記録する。

### TMI-006 [P0] schema-validとPDF-buildableを分けるcapability gateがない

**影響:** decoderだけを追加すると、schema-validなlist/table/figure等が深い`L5000`で失敗するか、意味を落として描画される危険がある。producerは実行前に利用可能なfeatureを判定できない。

**必要な改善:**

- `ValidatedParsedPackage`発行後、resource read/layout前に`MachinePdfCapabilityPreflight`を実行する。
- NodeId preorderでunsupported featureを決定的に収集し、上限内の全件をstable diagnosticとして返す。
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
- macOSでbuild/check/test/clippyとblank PDF smoke testを実行し、resourceを要求するfixtureがruntimeのstable unsupported errorになることを検証する。
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
  [--config CONFIG] [--resource-root DIR ...] [--strict] \
  [--trace TRACE.json] \
  [--emit-build-manifest MANIFEST.json] \
  [--emit-diagnostics DIAGNOSTICS.json]

typaxis check-package PACKAGE.json \
  [--package-root DIR] [--config CONFIG] [--resource-root DIR ...] \
  [--emit-diagnostics DIAGNOSTICS.json]

typaxis capabilities --format json
```

決定事項:

- `build`はreference TSF、`build-package`はDocumentPackageとし、extension/content sniffingをしない。
- `check-package`成功はSchema shapeだけでなく、trusted source validation、現在のPDF capability preflight、resource admission、style/font coverage成功を意味する。
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

sourceはpackage rootだけから解決する。font/imageは既存のadmitted resource-root集合から解決できるが、0 candidateはmissing、2以上はbytesが同じでもambiguousとする。producerがprivate job directoryだけを公開したい場合はpackage rootを唯一のresource rootにする。

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
  -> sealed machine-input admission
       -> stable package bytes
       -> package-root capability
       -> stable companion source bytes
  -> typaxis-syntax::DocumentPackageParser
       -> bounded private JSON decode
       -> private untrusted DTO
       -> source/text/document/style/resource validation
       -> entry-only or validated source-graph receipt
       -> ValidatedParsedPackage
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
        &self,
        input: &AdmittedMachinePackage,
        policy: &PackageValidationPolicy<'_>,
    ) -> ParseOutcome;
}
```

実際のfield/API名は実装時に確定してよいが、次の性質は必須である。

- callerはadmission receiptを組み立てられない。
- callerはdecoded DTOや`ParsedPackage`をtrusted packageへ直接promoteできない。
- package bytesとsource bytesの同一session identityを検査する。
- parse成功値は既存`ValidatedParsedPackage`であり、下流へ別の弱いpackage typeを増やさない。

### 6.6 検査順と副作用境界

同じ入力が同じprimary errorと副作用を持つよう、順序を固定する。

1. CLI syntax、target alias、config syntaxを検査する。
2. package rootとPACKAGEをcontained regular fileとしてstable-openする。
3. package byte limit、UTF-8、JSON lexical/depth/duplicate-keyを検査する。
4. contract、coordinate unit、root fieldをtyped decodeする。
5. source catalogの形、ID、URI、declared length/hashをpreflightする。
6. companion sourceをstable-openし、bytes/hash/UTF-8をadmitする。
7. source closure、SourceSpan、TextMap、node/style/master/resource semanticsを検査する。
8. `ValidatedParsedPackage`を発行する。
9. machine PDF capabilityをNodeId順にpreflightする。
10. font/imageをadmitし、style/font coverageを検査する。
11. layout、pagination、Display、resource finalization、PDF graphを作る。
12. PDF、trace、manifest、diagnosticsを既存atomic publication contractでcommitする。

step 9までunsupported contentのためにPDF temp fileを書かない。step 2以後にmanifest/diagnostics targetが成立した場合のfailed sidecar規則は、既存buildのterminal publication contractへ統合する。

### 6.7 capability artifact

`typaxis capabilities --format json`は少なくとも次を持つ。

```json
{
  "contract": "typaxis.contract/1.0",
  "engine": {
    "name": "typaxis",
    "version": "0.1.0"
  },
  "machine_input": {
    "profile": "typaxis.machine-pdf/paragraph-1",
    "source_closure": "entry_only",
    "blocks": ["heading", "paragraph"],
    "inlines": ["anchor", "hard_break", "reference", "soft_break", "text"],
    "font_formats": ["truetype-glyf"],
    "image_formats": [],
    "pdf_features": ["named-destinations", "text-extraction"]
  }
}
```

これは初期profileの形を示す例であり、現行CLIが出力できるという意味ではない。実装時は実際にlosslessなfeatureだけを列挙する。たとえば`link`はannotation生成まで、`figure`はimage paintまで、`emphasis`/`strong`は指定した視覚/semantic contractを保持できるまでadvertiseしない。

### 6.8 build manifest

推奨追加field:

```json
{
  "input_profile": "typaxis.machine-pdf/paragraph-1",
  "package_input": {
    "uri": "document-package.json",
    "bytes": 12345,
    "sha256": "<raw-package-sha256>",
    "canonical_sha256": "<typed-jcs-sha256>"
  },
  "inputs": [
    {
      "uri": "sources/book.json",
      "bytes": 67890,
      "sha256": "<source-sha256>"
    }
  ]
}
```

source modeにも`input_profile`を必須にし、`package_input = null`とする。manifestのSchema、Rust type、JCS encoder、minimal/conformance/invalid fixtures、validator、docsを一つのchange setで更新する。

### 6.9 diagnostic location

machine packageのsyntax errorはSourceSpanを作る前に発生するため、次のようなlocation unionが必要である。

```json
{
  "kind": "package_json",
  "uri": "document-package.json",
  "json_pointer": "/document/blocks/3",
  "byte_offset": 1942
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

`dump-ast` encoderはmachine profileで受理すると表明したfieldを完全にserializeする。受理対象外のpackage shapeを内部errorにするのではなく、command documentation上のexport profileを明示する。raw JSON bytesの一致ではなく、typed canonical JCSとDocumentFingerprintの一致をround-trip条件にする。

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
- `DocumentPackageParser`からsealed `ValidatedParsedPackage`を発行する。
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
