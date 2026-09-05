# VMB全巻PDFの互換性・リソース予算・診断改善設計

状態: Proposed（調査・設計のみ。製品コードの修正・全巻PDFの成功確認は未実施）
調査日: 2026-09-05
Typaxis baseline: `718ab6c9e1309b7dc750c62554c954cae4333131`
対象: `typaxis.contract/1.4` / `typaxis.machine-pdf/production-book-1`

詳細化: 2026-09-05。本書はTypaxis側の正本。VMB側の変更はユーザー指定の[VMB docs/typaxis-book-export-design.md](../../../v/vmb-container/docs/typaxis-book-export-design.md)を正本とし、処理境界・共通受け入れ条件のみ本書にも記載する。以下の「新設」「提案API」「次期」は未実装の設計を示す。

## 1. 結論と修正範囲

今回の章入力の直接原因は、複数パスではなく、`<path ... />` の **`/>` 直前の空白**である。実際の最初のSVGは、その空白だけを取り除くと複数パスのまま受理された。複数パスを一つへ結合したり、数式を画像リソースへ分割したりする修正は不要であり、描画順・穴・fill-ruleを変えるので採用しない。

全巻については、これだけでは解決しない。画像数4,514件に加え、ベクターの合計セグメント予算、SVGの物理寸法・座標系、和文フォント、数式の代替テキストが別々の障害になる。

採用する主経路は次のとおり。

1. Safe-SVG 2のタグ末尾空白を受理する限定的な互換修正と、SVG内部の原因を保持する診断を実装する。
2. production-book-1の未指定時の画像数を8,192、ベクターノード数を262,144、ベクター処理セグメント数を4,000,000へ変更する。既存のCLI/configによる明示指定を優先する。
3. VMBのTypaxis exporterでroot寸法・座標・metricsを一貫して出力し、実際の数式ごとの代替テキストを渡す。
4. 現行フォント対応範囲を正しく診断する。全巻の最初の実用ゲートは対応済みの日本語TrueTypeフォントを明示指定して実行する。
5. 原ノ味明朝の正式対応は、CID-keyed CFF1を扱う別のフォント拡張として実装する。現在の`sfnt-cff1/1`のtable whitelistを外すだけの修正では完了しない。
6. 実出力fixture、数百数式の章、5,000リソースの生成入力、実全巻を段階的に検証する。PDFの成功だけでなく描画・配置・抽出・構造の一致を完了条件にする。

「全巻を一つのDocumentPackageで生成する」と「提供された原ノ味明朝を無変更で使用する」を別の受け入れ項目として管理する。前者は章分割なしで必須、後者は§7の正式対応が完了するまで未対応と明示する。元の全巻packageを一切変更せず通すとは約束しない。元入力自身に§3のexporter不整合があるためである。

## 2. 調査資料と再現結果

### 2.1 実入力の保全

ユーザー提供元:

- VMB: `/Users/kazuyoshitoshiya/v/vmb-container`
- 書籍: `vmb-book-fractions-equivalence/v1`
- 全巻: `/private/tmp/vmb-fractions-typaxis-20260905d/document-package.json`
- 章: `/private/tmp/vmb-typaxis-ch00k/document-package.json`
- フォント: `vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf`

調査用コピーをリポジトリ内の`workspace/target/vmb-design/20260905/`へ保存した。`book-original/`と`chapter-original/`はpackage・source・resourcesのコピー、`book-statistics.json`は実全巻の統計、他のJSONは診断・比較実験結果である。これはローカル調査証跡であり、チェックイン済みfixtureやrelease evidenceではない。`target`削除の対象にもなるため、継続試験に使うデータは§8に従って追跡対象fixtureへ昇格する。

全巻packageのSHA-256:

```text
e4fd7415cb33de5afff8c3b2a335c2afa06df1e22780c9913813d95536efffbb
```

### 2.2 現行CLIで確認したこと

`cargo build --manifest-path workspace/Cargo.toml --package typaxis-cli --bin typaxis --locked`でbaselineのCLIを構築した。既存production combined packageの一つのSVGを差し替え、SHA-256も更新して公開`check-package`を実行した。SVGの構造を切り分ける実験であり、差し替えた数式のlayout/PDFの証明ではない。

| 入力・条件 | 結果 |
| --- | --- |
| 空白区切りの`M L Z`を持つ1パス | 成功 |
| `g fill="currentColor"`内の2パス / 1,000パス | 成功 |
| root直下の2個の`path fill="currentColor"` | 成功 |
| 一つの`d`に`Z M`で区切る複数サブパス | 成功 |
| 小数座標と`M L C Q Z` | 成功 |
| `M1 1L6 1L6 6Z`という省略記法 | `R7100 malformed_svg` |
| 実章の最初のSVGを無変更で差し替え | `R7100 malformed_svg` |
| 同じ実SVGの` />`だけを`/>`へ変更 | 成功 |
| 5,000画像宣言（既存SVGへのaliasを含む）、既定値 | `P1102` |
| 同じ5,000宣言に`--max-images 8192` | 成功 |

実章packageそのものも`/resources/images/0`で同じ失敗となった。最初のSVGのSHA-256は`3f1e47acffd70426c0376aafaae3331152cce202a7fe926236f52fc699fc90a0`。実章は物理pt座標・root寸法があり、全巻版のroot形状とは異なる。

省略記法の失敗は追加の互換性情報であり、今回の実章の原因ではない。現行仕様が明示的に空白区切りを要求しているため、この調査を理由に一般SVG grammar全体を受理する変更を混ぜない。

### 2.3 既存実装・試験の不足

- `typaxis-resource-admission/src/safe_vector.rs`の`MarkupScanner::next`は、区切り空白の後が`>`または`/`なら拒否する。`scan_v2`のroot/groupには1パス制約がない。1子要素制約があるのは`clipPath`である。
- 同ファイルの`checked` / `preserve_limit_or_malformed`は、非limitエラーの詳細を`MalformedSvg`へ潰す。scannerのbyte位置やpath番号も上位へ渡さない。
- `typaxis-core/src/lib.rs`の`ResourceLimits::default()`は`max_images=1024`。CLIの`config.rs`には`max_images`と`max_vector_*`の設定・引数処理がすでにある。
- 現行capabilitiesの`limits`にはpackage bytesとJSON nestingしかない。`max_images`を列挙していない。
- `typaxis-document-package/src/semantic_container.rs`の1.3 carrier再検証では、画像予算超過の元エラーをgeneric shapeエラーに包む経路がある。本調査の合成1.4 packageではJSON Pointerが空文字列になった。ユーザー報告の精密な`/resources/images/1024`が全経路で維持されるわけではない。
- 既存`precomposed-vector` corpusは13 logical resources / 12 distinct SVG、33 occurrencesの小規模証拠を持つ。分数等の図形は単純化されており、実フォントアウトラインの分布や数千リソースを証明していない。ledgerのproducer identityだけでは実VMB出力由来の証拠にならない。

## 3. 全巻データから追加で判明した問題

| 実測項目 | 値 |
| --- | ---: |
| package bytes | 10,672,653 |
| 画像宣言 | 4,514 |
| `svg-safe-2` / PNG | 4,466 / 48 |
| distinct SVG hash | 4,466 |
| 数式配置 | inline 7,866 + block 283 = 8,149 |
| SVG bytes合計 | 32,704,908 |
| path要素合計 / 最大 | 49,311 / 95 |
| XML要素合計 / 1 SVG最大 | 58,243 / 97 |
| 明示path command合計 / 1 SVG最大 | 1,010,441 / 2,445 |
| 最大SVG bytes | 82,997 |
| width・height両方が欠けるSVG | 4,466 |
| 最大viewBox値 / 最大path座標絶対値 | 3,373,098 / 3,362,382 |

実SVGのcommandは`M L C Z`で、各コマンドを明示している。単純なcommand数に各SVGの合成外周clip 5セグメントを加えると、少なくとも**1,032,771**の処理セグメントになる。現在の1,000,000を超える。これは実測構文からの予算計算であり、全巻が`R7121`まで到達した実行結果ではない。全巻を空白除去だけして検査した実験は、先にroot寸法欠落で失敗した。

全巻rootは次の形で、現在必須のwidth/heightを持たない。

```xml
<svg viewBox="0 0 45679 37880" xmlns="http://www.w3.org/2000/svg">
```

座標はproducer内部単位である。現行SVG parserは座標絶対値1,000,000以下を要求するため、大きな式には別の拒否要因もある。Typaxis側でrootの不足寸法を推測すると、同じSVGを複数のmetricsで使う場合のintrinsic sizeとhash共有の意味が壊れる。

追加のsource調査では、engineの`Metrics.OriginX`がviewport内部の原点位置であること、Ascent/Descentが外周Paddingを含まないことも確認した。Typaxisのorigin_xはviewportのペンからの差なので符号反転が必要で、Ascent/Descentはbaseline/viewportに合わせてPadding込みへ変換する。元packageの正のorigin_xをそのまま正解のgoldenにはしない。

さらに全inline数式7,866配置の`alt`が一律「数式」、`actual_text=null`だった。現行FormulaではnullのActualTextがAltへfallbackするので、抽出結果も数式の内容を表さない。TypaxisはTeXから読み上げ文を生成しないため、これはVMB exporterの修正項目である。

本文幅にも不整合がある。現在のA4縦・左右54ptの本文幅は487.2756195ptだが、inline数式のnode 1991 / 4216 / 21616（image 456 / 885 / 4187）は単独でも本文幅を超える。最大viewport幅は566.1632996ptである。これはmetricsとpage masterの静的比較であり、実buildのlayout診断まで到達した結果ではない。

全巻positive fixtureでは、数式を縮小・分割せず収めるため、A4横・左右54ptのpage masterを**入力として明示指定**する。元のA4縦packageはoverflow negative fixtureとして残す。実出版でA4縦を維持する場合は、VMBでの長い式の組み直し、または十分に広い本文領域を持つ出版レイアウトの選択が別途必要になる。Typaxisが検査中に用紙や数式のサイズを自動変更する設計にはしない。

## 4. Safe-SVG 2の変更

### 4.1 語彙を広げる範囲

Safe-SVG 2に限り、start tag/self-closing tagの最後の属性と`>` / `/>`の間のASCII SP・TAB・LFを0個以上受理する。root、group、geometryで統一し、`<path ... />`を素のbytesのまま読めるようにする。終端記号の途中の`/ >`、重複属性、未知属性、entity、script、external reference等の拒否は維持する。

`MarkupScanner`にprivateなlexical policyを渡し、`SafeSvg1`と`SafeSvg2`の選択を明示する。V2のための文字列置換やXML再serializeをadmission前に挟まない。expected SHA-256は提供された全stable bytesに対して検証し、そのbytesに対する位置を診断する。

`<g>`とrootに複数drawを順序通り蓄積し、pathごとに独立したpaint operationを維持する。`d`内の複数サブパスは一つのpaint operationのまま扱う。`currentColor`継承、fill-rule、穴、相対座標のcurrent point、close後のsubpath origin、Q/C制御点を保持する。root clipは既存の一回の適用を維持する。

`M/m L/l H/h V/v Q/q C/c Z/z`と既存の6桁までの小数grammarは本修正で維持する。今回実データにないcompact path、指数表記、`S/T/A`等は別の受理拡張とし、少なくとも`unsupported_path_syntax`または`unsupported_command`と該当tokenを診断する。普通のSVGとして妥当なものを一律「壊れている」と呼ばない。

### 4.2 公開仕様との関係

[ADR-0033](../adr/ADR-0033-math-safe-vector-and-alternative-binding.md)はタグ終端直前の空白を明示的に禁止し、[ADR-0037](../adr/ADR-0037-producer-composed-math-vector.md)がこれを継承している。したがって本件は単なるparser typo修正ではなく、**Safe-SVG 2の受理範囲に対する限定的な仕様訂正**として扱う。

実装開始時にADR-0038を追加し、V2のタグ末尾空白だけを既存凍結規則の例外として採択する。`svg-safe-2`と`production-book-1`は維持し、既存受理入力のIR・fingerprint・allocation chargeは変えない。新規受理入力のsource hashは空白込みで別物になる。`svg-safe-1`の受理/拒否・goldenは変えない。既存negative fixtureのうちV2空白禁止だけを要求するものは理由を記録して移行する。

この例外を一般化してCID-keyed CFFや新規JSON fieldを1.4へ後付けすることはしない。§7の正式フォント対応と§6のcapabilities構造拡張は別のversioned拡張である。

### 4.3 rootとproducer metricsの受け渡し契約

同じ縦横比のviewBoxと`pt`寸法は既存のexact-rational比較、固定小数点化、一つのuniform scaleで検証する。比較失敗を`aspect_ratio_mismatch`、固定小数点へ表現できない場合を`unrepresentable_scale`として区別する。x/y独立scale、黙ったcrop、metricsの自動書き換えはしない。

VMB側の具体的な変換API・丸め・metadata処理は[VMB設計§4〜6](../../../v/vmb-container/docs/typaxis-book-export-design.md)に定める。Typaxisの入力は物理pt座標のSVGで、rootの`width="Wpt" height="Hpt" viewBox="0 0 W H"`は同じW/H文字列を使用する。nodeの16.16 viewportはrootを再読込したintrinsic sizeと一致し、この経路のscaleは1となる。

受け渡しでは`origin_x = -physical_raw(engine.OriginX)`、`baseline = physical_raw(engine.Baseline)`とする。line ascent/descentはそれぞれbaseline以上、viewport.height-baseline以上へPadding込みで確定する。既存Typaxisのplacement式を変更してproducer側の符号誤りを吸収しない。

空白正規化のためだけに座標を変換することはない。座標変換は**全巻の欠落寸法・内部単位の解消**に必要なexporter処理である。元SVG/metricsと変換後SVG/metricsの画面上の点の差を測定し、誤差は一つの16.16物理座標単位以内とする。丸めで非空輪郭が潰れる入力は黙って出力せずexport errorにする。

### 4.4 scannerとpath処理の具体的な変更

変更先は`workspace/crates/typaxis-resource-admission/src/safe_vector.rs`。新設するprivate enum `MarkupLexicalPolicy::{FrozenV1, SafeV2}`を`MarkupScanner::new(bytes, policy)`に渡し、既存`decode`は常にFrozenV1、`scanner`/`scan_v2`は常にSafeV2を使う。callerが任意policyでreceiptを発行できるpublic APIは作らない。

`MarkupScanner::next`の区切り空白分岐を次の状態遷移に変更する。

```text
consume_sep()
next == '>'  && policy == SafeV2  -> Start tagを返す
next == '/'  && policy == SafeV2  -> 直後が'>'ならEmpty tag、他はunexpected_token
next == '>' or '/' && FrozenV1   -> 従来どおり拒否
それ以外                         -> 次のattributeを既存grammarで読む
```

EOFは`unexpected_end_of_input`。`</g >`や`/ >`は新たに受理しない。属性ゼロの`<g >`はlexingとして読めるが、empty group禁止等の構造検査は継続する。skipした空白はnode/segmentとして数えず、IRへ保存しない。

`Tag`にはstart/end byte、`Attr`にはname/value byte rangeを追加する。名前は既存の閉じた語彙へ解決し、未知名の診断にはspanを使う。`PathTokenCursor`に`token_start`を持たせ、各`next`で`d`全体からの相対offsetを記録する。親stackとは別にSVG全体のgeometry/path preorder counterを進め、clip内pathもpath番号へ含める。

Count/Analyze/Buildは同じvisitorを使用し、座標・arity・正負/relativeの判定をpassごとに別実装しない。Countではsegmentを一つずつstack上で処理し、上限チェック後にだけBuildのVecを確保する。各pathでcurrent point、subpath origin、segment indexをresetし、同一path中の新しいMだけでsubpath indexを増やす。

一般SVG grammar拡張は行わないが、既知のcompact表記は`unsupported_path_syntax`へ分類する。数字として読めたが範囲外なら`coordinate_out_of_range`、commandに必要なoperand数が足りなければ`wrong_parameter_count`、`A/S/T`は`unsupported_command`にする。どの分岐でも、最後に`malformed_svg`へ上書きしない。

### 4.5 canonical IRとPDFの検証点

同じgeometryのV2 compact-tag版とspaced-tag版は`SafeVectorIrV2.canonical_jcs()`・IR fingerprint・work countersが完全一致し、source bytes hashだけが異なることをassertする。resource fingerprintやmanifestまで同一であるとは要求しない。

PDF writerの変更は原則不要。実fixtureで既存writerがdraw順を保つか確認し、個別pathの`m/l/c/h`とfill/stroke終端、QのCへのlowering、rootのouter clipと一回のY flipを検査する。穴を持つ二つのsubpathを別fillに分割したり、独立pathを一つのfillにまとめたりする実装変更を禁止する。

## 5. 診断設計

### 5.1 resource-localなエラー型

`ResourceAdmissionError`からresource固有contextへ到達できるようにし、V2 parserの低水準エラーを単一enum値に置き換える処理を廃止する。contextは次を持つ。

| 情報 | 規則 |
| --- | --- |
| `reason` | 安定したtyped reason。構文、未対応機能、座標、上限、hash等を区別 |
| `element_index`, `element_name` | SVG内preorder、0始まり。UIメッセージには1始まりを明記 |
| `path_index`, `subpath_index`, `segment_index` | 適用可能な場合のみ。path番号はSVG全体で一意 |
| `attribute`, `token` | `d` / `viewBox` / `width`等。tokenはUTF-8境界で最大80 bytesに制限・escape |
| `byte_start`, `byte_end` | stable resource bytesに対する0始まりhalf-open範囲 |
| `line`, `column` | 1始まり、UTF-8 byte column。失敗時に一回導出しUTF-8文字数と混同しない |
| `budget`, `limit`, `observed`, `scope` | node / stored segment / clip replay / nesting / allocationを特定。document-totalかresource-localかも明記 |

最低限のreasonは`unexpected_token`、`missing_attribute`、`duplicate_attribute`、`unsupported_attribute`、`unsupported_command`、`invalid_number`、`coordinate_out_of_range`、`wrong_parameter_count`、`aspect_ratio_mismatch`、`unrepresentable_scale`、`budget_exceeded`を持つ。既存forbidden/external/hashの分類も保持する。

scannerはTag/Attrのspanを返し、path cursorは`d`内のtoken offsetを返す。割当て前Count passでも同じcontextを生成できるようにする。Count/Analyze/Buildの不一致は入力の`malformed_svg`ではなく、内部整合性エラー`I9190`として扱う。

### 5.2 既存diagnostics JSONへの投影

1.4のJSON shapeを変えず、主locationは引き続きpackage JSON Pointerとする。SVG offsetをpackageの`byte_offset`へ入れない。resource-local contextはmessageと`notes`へ表示する。notesのlocationは適切な既存型がない場合nullとし、TSF source spanを偽造しない。

例（内容を示すもので新JSON fieldの提案ではない）:

```text
R7100: svg_safe_2 missing_attribute: svg element 1 requires width
at document-package.json /resources/images/0
note: resource=resources/<hash>.svg; element=svg[1]; attribute=width;
      svg_byte=0; line=1; byte_column=1

R7121: max_vector_path_segments exceeded: observed=4000001 limit=4000000
at document-package.json /resources/images/4513
note: scope=document-total; resource=resources/<hash>.svg;
      path=28; segment=63; charge=stored_segment; svg_byte=18422
```

`R7100`をmessageとformatterの両方で付ける二重表示も解消する。codeは一つのownerだけが表示する。予算は既存`R7120` node、`R7121` segment、`R7122` nesting、`R7111` allocationを保持する。現在独立したpath数予算はないため、path番号の表示とpath数上限を混同しない。path要素はnode予算で課金する。

DocumentPackageの画像数エラーは1.3 carrier経由でも元の`Images`、limit、observed、`/resources/images/N`を保持する。`max+1`の最初の宣言で拒否し、resource openやlayoutを開始しない。契約版による既存exit statusの違いは本件で無関係に変更せず、コード・位置・原因情報を回帰試験で固定する。

### 5.3 エラー型とCLIまでの接続

現在の`ResourceAdmissionError`/`Cff1Error`は`Copy`で、CLIは同じ値をdiagnosticsとprocess failureへ二回渡している。これを維持するため、詳細contextはborrowed SVGやheap Stringではなく固定長の値とする。提案型の骨格は次のとおり。

```rust
struct ResourceByteSpan { start: u64, end: u64 }
struct DiagnosticToken { bytes: [u8; 80], len: u8, truncated: bool }
struct VectorBudgetFailure {
    kind: VectorBudgetKind, scope: BudgetScope,
    limit: u64, observed: u64, used_before_resource: u64,
}
struct SafeSvg2Failure {
    reason: SafeSvg2DetailReason,
    span: Option<ResourceByteSpan>,
    line: Option<u32>, byte_column: Option<u64>,
    element_index: Option<u32>, element: Option<SvgElementName>,
    path_index: Option<u32>, subpath_index: Option<u32>, segment_index: Option<u64>,
    attribute: Option<SvgAttributeName>, token: DiagnosticToken,
    budget: Option<VectorBudgetFailure>,
}
```

これらは`Clone + Copy + Debug + Eq + PartialEq`を実装する。URIはcontextに複製せずresource declarationから取得する。line/columnはparser途中ではNone、stable bytesが有効なerror boundaryでspanから一回補い、bytes解放後にも保持する。80 bytesのtokenはencode前の長さ制限とし、escape後も最大480 ASCII bytesに制限する。入力全文・任意path・source TeXを診断へdumpしない。

`ResourceAdmissionError`へ`SafeSvg2Detailed(SafeSvg2Failure)`と`Cff1Detailed(Cff1Failure)`を追加する。既存variantsと低位V1 APIは保持する。`Cff1Failure`は既存`Cff1Error`のkindと固定長`FontFailureContext`の組とし、`admit_sfnt_cff1_detailed`を新設する。既存`admit_sfnt_cff1`は同じ実装を呼びkindだけを返す互換wrapperとする。production admissionはdetailed APIを使用する。

projectionの順序は次のとおり。

1. parserはspan付きfailureを返す。budget failureにはremainingではなく、session全体のlimitとused_beforeを加えたobservedを境界で付与する。
2. resource resolverは同じfailureと既存`DiagnosticSubject::Resource(Image/FontFace)`を返す。URIは検証済みdeclarationから取得する。
3. `typaxis-cli/src/main.rs`の`production_resource_diagnostic_code`はmessage先頭をparseせず、typed kind→codeの全件matchへ変更する。
4. `emit_production_resource_diagnostic`は既存package locationでbuilderを作り、`DiagnosticNote::new`でresource/context/budgetを最大3 notesへ固定順に追加する。unknown index/spanは省略し、0で偽装しない。
5. `pipeline::map_public_resource_admission_error`にも同じcode/plain messageを渡す。JSON messageはcodeを含まない。stderr formatterだけが一回codeを付ける。V1の凍結artifact messageを変更する経路は避ける。

stable readが失敗してbytesがない場合はSVG位置なし。UTF-8不正はvalid_up_to byteのみを返し、文字列としてline/columnを生成しない。valid UTF-8ではエラー時一回のprefix走査でline/byte-columnを導出し、1 segmentごとの全SVG再走査を禁止する。

### 5.4 画像数予算エラーの修正箇所

`typaxis-document-package/src/semantic_container.rs`の`StagingSemanticDecodeError`に`ResourceCountLimit { axis, limit, observed, pointer }`を追加し、`pointer()`、Display、CLIのtyped diagnostic projectionへ接続する。axisはImages/FontFacesの閉じたenum、limit/observedはu64、pointerは元1.4 JSONに対するものとする。

1.4 rootのresourcesが配列であることを確認した直後、carrier生成前に宣言数を検査する。images長>Nならpointer=`/resources/images/N`、observed=N+1。font_facesも同様。既存duplicate ID/field構造検査は削除しない。strict JSON syntax/duplicate key→root shape→resource array shape/count→個別宣言という優先順をfixtureで固定する。

既存の`map_err(|_| Shape("unchanged contract-1.3 carrier is invalid"))`をすべてのエラーに適用しない。carrierでImages/FontFaces limitが発生した場合は同じtyped errorへ変換する。carrierはcanonicalizedな別bytesなので、そのbyte offsetを元package offsetとして返してはいけない。元location indexで取得できる場合だけoffsetを付け、なければnullにする。flattenされるdocument blockのpointerを単純に転写する変更は本件へ含めない。

## 6. リソース予算と共有

### 6.1 未指定時の値と解決順序

| limit | 現在 | production-book-1の提案値 | 課金単位 |
| --- | ---: | ---: | --- |
| `max_images` | 1,024 | 8,192 | logical画像宣言数。未使用・aliasも含む |
| `max_vector_nodes` | 100,000 | 262,144 | 文書内の各vector宣言に対する解析node合計 |
| `max_vector_path_segments` | 1,000,000 | 4,000,000 | stored segments + 外周clip + clip replayの合計 |
| `max_vector_nesting_depth` | 32 | 32 | resource内深さ |

画像8,192だけで任意に複雑な8,192数式を保証するわけではない。今回の全巻実測約103万セグメントに対し約3.8倍の余裕を設け、5,000件の実分布入力で検証する。現行hard maximum（nodes 1,000,000、segments 10,000,000、depth 64）は維持する。base bytes・decoded allocation・PDF objects・spool・outputの予算も引き続き独立に有効である。

`ResourceLimits::default()`をグローバルに書き換えず、`config.rs`のmerge開始前にprofile defaultを選ぶ。build/check両方が同じresolverを使い、順序は **profile defaults < config < environment < CLI** とする。明示的な1,024を「旧defaultと同じだから」と8,192へ置換してはいけない。`with_contract`後に値を上書きする実装も不可。最終effective limitsを一回検証・fingerprint化してdecode/admission/layout/PDFへ渡す。

今回の8,192への引き上げで画像数用の新CLIは不要である。現在でも次は利用可能。

```sh
typaxis check-package document-package.json \
  --package-root . --resource-root . \
  --profile typaxis.machine-pdf/production-book-1 \
  --max-images 8192 --max-vector-path-segments 4000000
```

同じ引数を`build-package`へ渡せる。これは数・処理予算の指定例であり、現状の実全巻SVGやCFFを直す回避策ではない。configでは`[limits] max_images = 8192`等の既存キーを使用する。0・整数型範囲外・既存hard max超過・相互矛盾をconfig段階で拒否する。

### 6.2 hash共有と大規模入力

現在のsource bytes SHA-256検証、content keyによるForm共有、logical IDとprovenanceの保持を維持する。違うbytesを「見た目が同じ」と推測して統合しない。空白の有無でsource hashが変わることも維持する。

現行契約では同内容のaliasでも宣言ごとのstable-byte検証とIR admission workを省略しない（ADR-0037）。Formが共有されても`max_images`やparser workが無料になるわけではない。この変更で課金単位をunique hashへ暗黙に変更しない。

大規模試験は「5,000件すべて同じSVG」だけでは不足する。distinct 5,000、alias混在、実際の4,466 distinct / 8,149 placementsを別々に扱う。Form数は選択されたdistinct content key数、page `Do`数は配置数と一致させ、共有FormへAlt/ActualText/MCIDを置かない。

`parse_and_bind_declared_safe_vector`には以前の宣言全件を毎回走査する処理があり、`AdmittedResourceLedger::image`にも線形探索がある。§6.5のcursor/indexで置換し、5,000件で時間・RSS・read回数を比較する。最適化しても順序拒否、receipt、出力の決定性を維持する。

### 6.3 capabilitiesの扱い

既存`capabilities --format json`はconfigを読まない固定記述である。将来のcapabilities拡張ではprofileごとに`default`、`maximum`、`scope`、`configurable`を示し、最低限images/nodes/segments/depth/decoded bytesを公開する。画像数の`maximum`は実装が検証する範囲を示し、実用処理保証と区別する。実際のoverride値はeffective config / manifestで観測する。

ただし公開済み1.4 capability schemaは`additionalProperties:false`で、profile/limit shapeも固定である。**既存1.4 JSONへ新fieldを無断追加しない。** capability shape拡張は次期contract registryの公開時にSchema・encoder・golden・producer guideを同時更新する。本互換修正の必須完了条件には含めず、当面は既存CLI/configとproducer guideで値を明示する。profile defaultsを変更した実装にはresolverの単一sourceから生成する文書・試験を用意し、capabilities追加時も同じ値を投影する。

### 6.4 config resolverの提案APIと境界試験

`typaxis-core/src/lib.rs`に`MachineResourceDefaults::for_profile(MachinePdfProfileId)`を新設し、base/extensionの値を返す。production-book-1だけ§6.1の3値を上書きする。他profileとsource buildは従来defaultを使う。

`config.rs`に`load_for_profile(profile, config_path, environment, overrides)`と`load_from_process_env_for_profile`を追加する。既存`load`はsource/legacy defaultを使うwrapperとし、既存test callerの意味を変えない。`MergedConfig::for_profile`はmerge前に値をセットし、`finish`は同じEffectiveConfig constructorで相互制約を検証する。

`main.rs`の`load_config`をprofile引数を受け取る内部helperへ分け、`run_check_package`とbuild-package双方がCLIで選ばれたprofileを渡す。未知profileはoption parseで拒否し、raw package contractを見てdefaultを後から選び直さない。既存`with_contract`はartifact dispatchに残せるが、limit値には触れない。

必須config試験は、無指定8,192、configの1,024維持、`TYPAXIS_LIMITS__MAX_IMAGES=2048`がconfigに優先、`--max-images 5000`が環境に優先、source/旧profileが1,024のまま、check/build effective fingerprint一致。max_vectorも同じ試験を行う。0、u32 overflow、vector hard max+1、image bytes>resource bytesを失敗させる。

実行時の最初の8,193画像はSVG parse前に拒否される。5,000画像成功試験は設定flagなしで実行する。明示的な高いlimitを持つ旧入力をdefault変更と混同しない。

### 6.5 性能修正の決定内容

同じ5,000件を複数回走査する箇所は、本件で次の二つに限定して修正する。

- resolverに`next_vector_declaration_index`を追加し、初期化時と一つのvector admission成功時だけ次のvector宣言まで進める。PNG/JPEGをskipする。対象IDがcursorと違う場合は従来のReceiptIdentityMismatch。失敗では進めない。全宣言prefixを毎回探索しない。
- `AdmittedResourceLedger::image`はfinalization時に全dense image IDが順序通りそろうことを検証したうえで`images.get(id as usize).filter(id一致)`を使う。部分admission receiptや未確定mapはこのledgerと区別し、欠番があり得る途中状態へ直接indexを適用しない。

パーサの三passとaliasごとのwork課金、BTreeMapに基づくartifact順は維持する。時間短縮のために署名済みreceiptを別宣言へ使い回さない。前後の出力hash・診断順とout-of-order拒否を比較する。

### 6.6 次期capabilities JSONの具体形

§9.1の1.5 registryでprofile entryへ`resource_limits`を追加する。例は`production-book-1`のimagesだけを示す抜粋で、独立したtop-level JSONではない。

```json
{"resource_limits":{"max_images":{"default":8192,"maximum":4294967295,"scope":"document-declarations","configurable":true}}}
```

max_imagesのmaximumは現行u32の検証上限で、実用保証件数ではない。他budgetも同時に満たす必要がある。nodes/segments/depthは既存hard max、decoded bytesはそのfieldの検証域と相互制約の説明を使う。scopeは`document-declarations`、`document-vector-nodes`、`document-vector-work`、`resource-depth`、`resource-decoded-allocation`の閉じた語彙とする。

defaultは§6.4のownerから生成し、capabilities専用の定数を持たない。configurableはCLI/configに実際のキーがあるものだけtrue。例に含まれないschemaの全必須fieldも同時に出力する。current capabilitiesはconfig/環境を読まないままとし、overrideを知るためのprobeとは扱わない。1.4 registry/goldenは残し、1.5 aliasの変更だけを公開する。

## 7. 和文フォント

### 7.1 原ノ味明朝の拒否理由

VMB同梱ファイルの実測:

```text
SHA-256: 66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717
bytes: 6,422,896
OS/2.fsType: 0
glyphs: 23,060
CFF.ROS: Adobe / Japan1 / 7
FDArray: 18 entries
FDSelect: format 3
cmap formats: 4, 12, 14
```

同じ実ファイルに公開`admit_sfnt_cff1`を直接呼ぶ一時的な調査プログラムを作り、`Cff1Error::UnsupportedTable`を確認した。sfnt directoryの順序とwhitelistを照合すると、最初の未対応tableは **`VORG`**。`vhea`と`vmtx`もwhitelist外である。

原因の切り分けのためにコピーからこの3tableを除いたところ、次は`InvalidCmap`となった。cmap format 14も現行parserが受理しない。これは診断実験であり、tableを削除したフォントを本番入力にする提案ではない。

さらに現行`validate_top_dict`はROS/FDArray/FDSelectを許可しない。[ADR-0036](../adr/ADR-0036-jpeg-and-opentype-cff-resource-profiles.md)の対応範囲はstandalone **name-keyed CFF1**で、CID-keyed inputは明示的に対象外である。したがって「VORGを許可すれば原ノ味対応完了」ではない。`fsType=0`なので今回のファイルの最初の拒否は埋め込み権限によるものではない。

### 7.2 先行する診断改善

フォントエラーにphase（sfnt directory / cmap / CFF dict / charstring / permission / subset）、table tag、table-relativeおよびfile-relative offset、face index、CFF operator/FD/GID、typed reasonを保持する。

埋め込み権限は`OS/2.fsType`の数値と判定を別に表示する。必要なtableまで安全に読めなかった場合は「未検査」とし、「権限に問題あり」と推測しない。structural malformed、valid but unsupported、restricted embedding、resource budgetは別reasonにする。

現行の日本語フォント形式を次のように案内する。拡張子やfont family名だけで合格とはしない。

| 宣言media | 現行対応範囲 |
| --- | --- |
| `sfnt-truetype-glyf` | standalone TrueType `glyf`、必要な日本語glyph/cmapと埋め込み条件を満たすface |
| `ttc-truetype-glyf` | TTC内の対応TrueType faceを明示選択 |
| `sfnt-cff1` | standalone name-keyed CFF1のみ。原ノ味明朝のCID-keyed入力は未対応 |

既存CLIの`inspect-font FONT`が返すface一覧を活用する。TTCの安全なheader/directory検査でface数が得られた場合は、resource診断に`requested_face_index`、存在する0始まりindex一覧、選択faceのoutline種別を示す。「存在するface」と「admissionできるface」は別の情報とする。TTC header自体が壊れている場合はindex一覧を推測しない。CFFの`face_index=0`限定とTTC TrueTypeの選択を混同しない。

### 7.3 原ノ味明朝の正式対応

別のversioned CFF resource profile `typaxis.resource-profile/sfnt-cff1/2`として次を一体で実装する。公開先は§9.1のcontract 1.5 / production-book-2とする。既存`/1`にCIDをname-keyedとして押し込まない。実装時ADRに以下の識別子と規則を登録し、全巻ゲートを通すまで正式対応を宣言しない。

1. `VORG`、`vhea`、`vmtx`を構造・offset・glyph数・metric数付きで検証する。水平組版で使わない情報も「無条件に信頼して読む」ことはしない。
2. cmap format 14を検証し、Unicode variation sequenceのcoverageと選択を定義する。収録を認めるだけの段階と、IVSを正しくshapingする段階を区別し、未対応のIVSを黙って基底文字に落とさない。
3. CID-keyed Top DICT、ROS、CID charset、FDArray、FDSelect 0/3、各FDのPrivate DICT/local Subrsをboundedに読む。FD・subroutine・CharStringsのoffset、重複、範囲、glyphとの対応を検証する。
4. Type2 evaluatorでGID→FD→local Subrsを選び、global/local bias、hintmask/cntrmask、width、stack/call depth/operation budgetを守る。global subroutineからlocal subroutineを呼ぶ場合も元glyphのFD contextを保持する。
5. 初期`/2`のCID入力はhead.unitsPerEm=1000、Top FontMatrix=`[0.001 0 0 0.001 0 0]`、FD FontMatrixなしに限定し、異なる行列は`unsupported_font_matrix`で拒否する。この条件は実原ノ味明朝に一致する。単一Private DICTを全glyphへ使う仮定を除く。
6. 既存のselected glyph closureとdense CID subset、FontFile3/OpenType・CIDFontType0・ToUnicodeへの接続を維持する。source CIDをUnicodeとみなさない。変更されたadmission/evaluator/subset/manifest identityはversionを上げる。
7. 実原ノ味明朝、複数FD・subroutines・IVS・句読点・日本語本文・不正offset・権限拒否・上限境界をfixtureにし、subset後の独立parse、レンダリング、抽出を検証する。

この作業は診断の小修正より大きいため、実用化の先行ゲートから分離する。原ノ味指定の全巻ゲートはこの拡張の必須完了条件とし、TrueType版の成功を流用しない。

### 7.4 CFF program・評価器の変更構造

実原ノ味明朝ではFDごとにdefaultWidthX/nominalWidthX/local Subrsが異なり、FD 12だけで21,626 local subroutinesを持つ。Top FontMatrixは0.001の対角、FD matrixは18個すべて省略。format 14には17 selector、14,780 UVS recordsがある。現行`CffProgram`の単一local_subrs/widthでは表せないため、`typaxis-font/src/cff_v2.rs`を新設し、V1型を変更せず次のprivate型を用いる。

```rust
struct CffFontDictV2 {
    private_span: ResourceByteSpan,
    local_subrs: Vec<ResourceByteSpan>,
    default_width_x: i32, nominal_width_x: i32,
}
struct CffProgramV2 {
    source: std::sync::Arc<[u8]>,
    charstrings: Vec<ResourceByteSpan>,
    global_subrs: Vec<ResourceByteSpan>,
    font_dicts: Vec<CffFontDictV2>,
    fd_by_gid: Vec<u8>,
    cid_by_gid: Vec<u16>,
}
```

ここでfont側の`ResourceByteSpan`はfont crate所有の同等のbytes-range型で、resource-admissionへの逆依存を作らない。元bytesを一つ保持してrange参照し、全programのVec<Vec<u8>>複製を避ける。name-keyed入力はFD一個、全fd_by_gid=0として同じ評価器へ接続できるが、元glyph SID/CIDの意味は別variantで保持し混同しない。

FDArrayは1〜256。FDSelect format 0はglyph数分、format 3はfirst=0・strictly increasing ranges・sentinel=glyph_count・各FD<FDArray.lenを検証して一回だけdense fd_by_gidへ展開する。CID charsetは形式0/1/2のGID→CIDとして解析し、SID/string INDEXを引かない。glyph 0、重複CID、overflow、CharStrings数とmaxp数の不一致はtyped failureとする。

新しい`evaluate_glyph_v2`は最初にfd_by_gid[gid]を取り、評価終了までFDを保持する。`ProgramKindV2::Local { fd, index }`と`Global { index }`を区別し、global→local callでもそのglyphのFDを利用する。widthは選択FDのdefault/nominalで解決し、hmtxとの整合を検証する。operator/stack/call-depth/stem/operation/outline予算はV1と同じ上限を継承する。

subroutine上限はglobal数 + 全FDのlocal数のchecked合計に一回適用する。同じrangeを複数FDで参照する場合も宣言ごとに数え、local INDEX数だけで許容量を増やさない。採番・評価順はFontFaceId→GIDの昇順。cache keyは`(profile_id, source_sha256, face_index, gid)`で、異なるFDやprofileの結果を流用しない。

`Cff1AdmissionV2`と`Cff1SubsetSessionV2`はprivate fieldのsealed receiptを返す。選択済みGIDだけを評価し、`.notdef`→0、残りをsource GID昇順にdense subset GID/CIDへ割り当てる。複数font instanceを跨ぐ共有と一回課金は既存ownerに従う。未選択glyphのprogramはoffset/INDEX構造まで検査し、全charstringを最初から実行しない。

### 7.5 追加table・cmap・IVSの検証

`VORG`はversion 1.0、8-byte header、record数に応じた長さ、glyphIndexの昇順/範囲を検証する。`vhea`は対応versionと36-byte構造、numberOfVMetricsを検査し、`vmtx`と対で存在することを要求する。`vmtx`長は`4*n + 2*(glyph_count-n)`と一致させる。水平組版では検証結果をoutline位置へ適用しない。VORGの情報を水平baselineとして使用しない。

format 14はplatform 0 / encoding 5の補助tableとして受理する。base cmapは既存4/12から選び、14をUnicode→GIDの代替mapにしない。selectorはFE00〜FE0F/E0100〜E01EF、昇順・重複なし。default ranges/non-default mappingsのoffset・length・Unicode scalar・重複/交差・GID範囲を検査する。ゼロoffsetは欠如でありtable先頭を読まない。

初期`/2`の固定防御上限はFD=256、UVS selector=256、UVS defaultの展開codepoint数+non-default mapping数=1,000,000とする。設定で増やさず、超過は`R7100 unsupported_font_complexity`にlimit/observedを添える。CFF/bytes/glyph/subroutine等の既存可変予算は別に適用する。

admissionは`VariationCoverage::{Default, NonDefault(gid), Missing}`を提供する。`typaxis-shaping/src/lib.rs`のpreflightでbase+VSを同一source clusterとして照合し、Missingではglyph coverage errorを返す。harfrustへは元二scalarをそのまま渡し、独自の文字置換をしない。defaultの場合もVSを消さず、ToUnicode/ActualTextには元の二scalarを保持する。孤立VSや未対応pairの黙った削除は認めない。

shape結果→selected GID→subset mapping→ToUnicodeのjoinをIVS fixtureで検証する。特に同じGIDに複数のUnicode sequenceが対応する場合は既存ActualText所有規則へ従い、CID→単一文字を推測しない。FontFile3/OpenTypeとCIDFontType0を継続し、TrueTypeのFontFile2やCIDToGIDMapをCFFへ使わない。

### 7.6 詳細フォント診断とface一覧

`FontFailureContext`は`phase`、`table_tag:[u8;4]`、file/table offset、requested face、optional GID/FD/operator、`EmbeddingStatus::{NotChecked, Allowed(fs_type), Denied(fs_type)}`を保持する。sfnt directory検証と各tableの範囲検証の後、安全に読めるOS/2があればpermissionを評価する。table不足/破損で到達不能ならNotChecked。primary failureは最初の失敗を保持し、後のparse失敗で上書きしない。

元原ノ味の旧profileでのexpected noteは`table=VORG; phase=sfnt-directory; reason=unsupported_table`、新profileの権限拒否fixtureは`table=OS/2; phase=embedding-permission; fs_type=...; status=denied`である。バイナリのunsupportedとmalformedは別reasonにする。

`typaxis-cli/src/font.rs`の既存`inspect-font FONT`はJSONを返し、face_count/faces/face_indexを持つ。そのshape・最大4,096 faces/file等の既存上限は維持する。通常のresource failure noteには最大32個のface index、総face数とtruncatedを示し、全一覧の取得コマンドを案内する。outline type/admission statusは選択faceのresource診断で示す。TTC header破損では一覧なし、存在するがCFF/variable/color等で非対応のfaceは「存在する／非対応」と区別する。新profileもTTC内CFF、CFF2、可変・color fontは今回の対応範囲外とする。

## 8. 実VMB結合テスト

### 8.1 fixtureの構成

新規`vmb-book` corpusには、実章の失敗SVG、全巻の最大パスSVG（image 4187）、最大セグメントSVG（image 2987）、分数・括弧・同値・否定・長い式をraw bytesで収録する。元source TeX、VMB engine/version・exporter revision、原始SVG hash、export後SVG hash、metrics、spacing、alt/ActualText、期待する視覚基準をledgerで結ぶ。合成pathを実VMB出力と表記しない。

現在のraw内部単位SVGと、修正exporterが出す寸法付きSVGの両方を保存する。前者はexporter変換試験・missing-attribute診断試験に使い、後者をTypaxisのpositive fixtureにする。章の` />`付きSVGは無変更のpositive fixtureへ昇格させる。

fixtureの採用時にVMB書籍・数式データ・フォントの出典と配布条件を記録する。ローカルArial Unicodeや書籍全巻を無条件にrepositoryへ再配布しない。通常のローカル試験は配布可能な固定フォント・抽出した数式コーパスを使い、実全巻/実フォントはhash固定した明示管理ホストの入力として別ゲートを持つ。

### 8.2 試験マトリクス

| レベル | 入力・試験 | 主なassertion |
| --- | --- | --- |
| parser | group/root複数path、単一d複数subpath、M/L/C/Q/Z、6桁小数 | draw/segment順・数、Q/C制御点、fill-rule、currentColor、同じcanonical IR |
| parser | `/>` / ` />` / TAB / LF、root/groupの末尾空白 | V2の同じIR、V1既存goldenと拒否を維持 |
| parser | 実測95 paths・2,445 segments以上、合成1,000 paths | 割当て前CountとBuildが一致、境界で正しいlimit |
| geometry | 同比率のviewBox/pt、非整数比率、負のmin-x/min-y、丸め境界 | uniform scale、正しいBBox、異比率の詳細拒否 |
| negative | 不正token/coordinate/arity、empty/move-only path、unsupported command、entity/script/external | code + reason + resource/attribute/path/offset、成功PDFなし |
| CLI小規模 | 実inline/block、分数・括弧・同値・否定・長い式、隣接和文 | checkとbuildの両方、配置・抽出・タグを比較 |
| 章 | 実fixtureから300〜500数式配置、行末・改ページ・数式番号 | 改行前後のbaseline/spacing、欠落・二重配置なし |
| overflow | 実全巻の幅超過3式と元A4縦本文幅、明示A4横本文幅 | 前者はL5100で位置を報告、後者は元数式サイズのまま配置 |
| 書籍規模 | distinct画像5,000、PNG混在、別途alias混在、複数章を一package | 既定値でcheck/build、resource数と配置数の別検証 |
| 予算境界 | images 8,192 / 8,193、設定1,024 / 1,025、nodes/segments/depth exact / +1 | inclusive上限、最初の超過resource、入力順の決定性 |
| 全巻 | 修正exporterによる実書籍、4,514リソース・8,149数式配置を基準に元入力と照合 | 一package→一PDF、全章・数式・図版・本文の一致 |

8,192画像境界試験は小さなSVGで構成して、先に別予算が尽きないようにする。segment境界試験では外周clipとreplayを含む実課金値を使用する。5,000画像試験は少なくとも5,000宣言を実際に配置し、「大量の未使用宣言がparseできた」だけでbuildの成功を代替しない。

### 8.3 PDFの観測方法

- **欠落・重複**: source node→selected placement→Display→page Do→structure MCRをjoinし、一対一対応を確認。Form数とDo数を混同しない。
- **切断・見た目**: 各FormのBBox、root clip、viewport matrixを独立検査し、元VMB SVGの同じ物理寸法でのrenderとPDF renderの数式領域を比較する。透明部分を含むmarginも検査し、単にPDF objectがあるだけでは合格にしない。renderer version、DPI、pixel toleranceは固定し、fixtureが失敗するから閾値を緩めない。
- **baseline/空白**: `viewport_top = line_baseline_y - baseline`、`viewport_left = pen_x + origin_x`、advance、spacing before/afterをtraceから検証する。行頭・行末のspacing抑制、和文隣接、連続数式、descent、番号とのgapも含む。
- **抽出**: VMBが渡した意味のあるActualTextまたはAlt fallbackが数式の出現順・回数で得られることを独立extractorで比較する。一律「数式」は内容の正しさのgoldenに採用しない。TeX source spanの一致も別に検査する。
- **タグ**: Formula/Figure、Alt、ActualText、Lang、MCID、ParentTree、読み順、式番号の独立Spanを検証する。同一Formの複数利用でも各配置の意味情報を分離する。
- **決定性/失敗**: 同じbytes/configで2回buildしPDFとmanifestの規定hashを比較する。limit/parser/font failure時に部分PDFを成功として公開しない。

既存`tools/verify_precomposed_vector.py`、`verify_pdf_structure.py`、PDF differential verifierを拡張して再利用する。内部trace照合だけで視覚的正しさを証明しない。外部renderer/extractor/veraPDFは既存tool policyの固定版でローカルまたは明示管理ホストにて実行し、GitHub Actionsは使用しない。

### 8.4 性能の観測とゲート

check/build別の経過時間、peak RSS、stable read bytes、宣言/unique/Form/Do数、segment charges、PDF bytes/pagesをJSONに記録する。1,000→2,500→5,000の同じ分布で増加率を比較し、明らかな二乗走査を特定する。最初の修正実装で測定した管理ホストbaselineから、同一ホストの時間/RSSが20%以上悪化した場合を要調査とする。未測定の秒数やメモリ使用量を既達成のSLOとして宣言しない。

### 8.5 fixture台帳とrunnerの固定仕様

新規ディレクトリは`samples/machine-package/staging/production-book-1/vmb-book/`とする。`fixture-index.json`は`algorithm="typaxis.vmb-book-fixture/1"`を持ち、全fileの相対path/byte_length/sha256、原VMB source/engine/exporter identity、case ID、license/provenance記録を収録する。期待値はgenerated PDFから自動採取せず、source/metricsと承認済みreferenceから作る。

`cases.json`の各caseは`case_id, source_tex_file, source_svg_file, admitted_svg_file, source_svg_sha256, admitted_svg_sha256, metrics, spacing, alt, actual_text, language, expected_path_count, expected_stored_segments, expected_path_work`を持つ。`occurrences.json`は`node_id, image_id, case_id, placement_kind, source_file, source_pointer, source_ordinal`で配置を列挙する。JSONはstrict decode、相対pathのcontained read、JCS+LFで固定する。VMB側sidecarとcase hash/metricsでjoinする。

5,000 distinct画像の試験は、実SVGのhash集合だけを反復する試験と区別する。固定template `x_{i}=\\frac{i}{i+1}`（i=1〜5,000の十進展開、11pt）を実VMB engineへ渡し、数値を含むtemplate由来のsemantic speechも生成する。全5,000 distinct SVGをassertし、XML空白だけで水増ししない。PNG混在caseは同templateのi=1〜4,952と48 distinct PNGで計5,000宣言とする。別のalias試験はcase集合を反復し、5,000宣言と8,000以上の配置を作る。各node ID/TeX/SVG由来を追跡する。

`typaxis-cli/tests/cli_end_to_end.rs`へ`vmb_book_*`の公開CLI試験を追加し、共通generatorは`typaxis-testkit`またはfixture専用Python helperに置く。parserの小fixtureはresource-admissionのunit test、full buildはCLI integration test、構造・render/extractionは新規`tools/verify_vmb_book.py`が所有する。

新規runnerのインターフェースは次を固定する。

```sh
python3 tools/verify_vmb_book.py \
  --typaxis workspace/target/debug/typaxis \
  --fixture-root samples/machine-package/staging/production-book-1/vmb-book \
  --output-root workspace/target/machine-e2e/vmb-book \
  --mode fixtures
```

`--mode book --package-root DIR --expected PATH`で管理ホストの全巻も同じ検査を行う。expectedにpackage/config/font/全resource hashとsource occurrence inventoryを必須化する。mode bookはfixtures modeの成功を省略する理由にならない。runnerはcase数と実行test名の集合を検査し、filterが一致せず0 tests成功となった場合を拒否する。

出力はcase別の`check-diagnostics.json`、`build-diagnostics.json`、`output.pdf`、`trace.json`、`manifest.json`、`observed.json`と、全体の`artifact-index.json`。observedは期待値とは別で、command argv、tool version/hash、return code、各phase成功、計数、時間/RSS、renderer/extractor結果を持つ。元full packageは書き換えず、生成入力はoutput-root内だけに作る。

### 8.6 独立描画検査とoracleの誤りを防ぐ条件

PDFレンダリングは既存external-tool-policyのMuPDF/Poppler、72/144/288 DPIを使う。SVG側reference生成はVMBが使用する固定版librsvgを含む新しいVMB-book専用tool-policyへbinary/version/source hashを記録する。既存の凍結tool-policyを書き換えて以前のevidenceを同じIDで再利用しない。raw internal-unit SVGはVMB geometry contractで物理寸法を与えてreference化し、derived SVGだけをoracleにして変換ミスを見逃さない。

許容差は以下を固定する。SVGとPDFの各数式を同じviewport位置へ整列し、白背景・black currentColor・同じDPIで比較する。前処理で画素サイズ変更や自動位置合わせをしない。

- exact geometry: IR/control pointの差は1/65536 pt以下、BBox/placement matrix/clip/Do数は規定値と一致。
- mask: luminance<128をinkとする。両maskのinkの各点が、他方maskの1 pixel以内に存在する。距離1 pixel超のmissing/extra inkは1点でも失敗。
- area: ink pixel数の差が`max(4 pixels, reference inkの1%)`を超えたら失敗。reference非空でPDF空は無条件失敗。
- whole-form crop: viewport外側2 pixelのringも比較し、不正な切断/はみ出しを検出する。内部の白抜き穴をmask比較から除外しない。

1 pixel以下の細線等はmask単独では証明できないので、原path/control pointとpaint operationの完全なjoinを併用する。small corpusには分数線・否定斜線を意図的に削除/二重描画/clipしたtampered PDFを用意し、verifierが必ず拒否することをtestする。ツール差異で閾値を変更する場合は新しいtool-policy/expectation identityと再レビューが必要。

全文抽出はNFC化や空白collapseで都合よく一致させず、既存規則が許す改行差だけをnormalizeする。数式occurrenceごとのActualText順序/回数を構造からも照合し、同じ「数式」という文字列が多数抽出されたことを内容一致の証拠にしない。PDF/UA gateはveraPDF 1.30.2と既存独立structure検査の両方を要求する。

## 9. 実装順序と変更owner

| 順序 | 作業 | 主なowner / 完了条件 |
| --- | --- | --- |
| 1 | 実データfixture固定・仕様訂正ADR | 新corpus、ADR-0038、docs/27追補。今回の再現と元hashを保存 |
| 2 | エラー詳細保持・carrier error伝播 | resource-admission、document-package、diagnostics、CLI。原位置・原因がJSON notesとstderrへ到達 |
| 3 | V2タグ終端空白 | safe_vector scanner policy。raw実章SVG成功、旧V1/V2受理入力のgolden不変 |
| 4 | profile defaultsと大規模予算 | coreのprofile default owner、CLI config resolver、decode/admission。override優先・5,000件試験 |
| 5 | VMB exporterの寸法/単位/意味情報 | VMB Typaxis backend。全巻SVG・metrics・altを同じ入力から再生成 |
| 6 | 実章・5,000件・TrueType全巻ゲート | CLI E2E、Python verifier、管理ホスト。単一packageの検証・PDFと独立検査が成功 |
| 7 | 原ノ味正式対応 | 新CFF profile/次期contract ADR→font/admission/shaping/resources/PDF/manifest→原ノ味全巻ゲート |
| 8 | capabilities拡張 | 次期registry公開時にSchema/encoder/golden/guideを一括更新 |

2・3・4はそれぞれfocused regressionを通し、6の前に結合する。5が未完了なら全巻成功を主張しない。7・8を先行する小修正へ混ぜない。

本設計は製品実装の完了記録ではない。既存docs/25・27のCompletedは旧corpus/旧受け入れ条件に対する実装完了記録として保持し、本件の全巻対応が完了した意味に書き換えない。

### 9.1 二段階の公開と識別子

先行修正はraw contract 1.4 / production-book-1のまま公開する。ADR-0038はV2タグ末尾空白の限定訂正、production profile default変更、診断詳細化を正当化する規範追補を持つ。旧profileのdefault・SVG1・1.0〜1.3 artifactは変更しない。1.4 JSON fieldは増やさず、message/notesと設定値で表せる修正だけを含む。

原ノ味正式対応とcapability shape拡張は次の登録を**同じ1.5公開ゲート**で行う。これらは今回作成する設計上の予約名で、現行CLIで使用可能な名前ではない。

| owner | 次期identity / 規則 |
| --- | --- |
| raw/current artifact contract | `typaxis.contract/1.5` |
| 新production profile | `typaxis.machine-pdf/production-book-2`。raw 1.5のみ |
| 旧production profile | production-book-1はraw 1.4・artifact 1.4・CFF `/1`のまま |
| CFF wire media | 1.5でも`sfnt-cff1`。profileのversioned admissionが対応範囲を決める |
| CFF component | `typaxis.resource-profile/sfnt-cff1/2` |
| CFF admission/evaluator/closure/subset/PDF plan | 既存各identityの`/2`。旧`/1`receiptとは型・fingerprint domainを分ける |
| production resource set | `typaxis.production-book-resource-set/3`。resource-set `/2`のCFF `/1`だけを`/2`へ置換 |
| vector media/component | `svg-safe-2` / safe-vector `/2`を再利用。空白訂正も含む |
| next capability record | contract 1.5、9 profiles、各profileのresource_limits |

production-book-2の既定予算はproduction-book-1の修正後と同じ。明示profileとraw contractが合わなければresource open前に拒否する。VMBは1.5 packageを1.4として送信せず、原ノ味の受理失敗で旧profileへ再試行しない。

実装ownerはcoreのcontract/profile enum、document-package decoder、machine-profile descriptor/capabilities、syntax/resource-admission、font/shaping、resources/PDF、manifest、CLI version dispatch、`schemas/1.5/`、top-level aliases、samples/producer guide。既存29-schema registryを1.5として独立に作り必要な追加shapeを定義し、1.4以下は元bytesを保持する。current output alias、canonical config、capabilitiesと新profileの公開は一つのchange setで行う。

新CFF `/2` receiptを受け取れない旧manifest/PDF ownerへcastしない。profile別のreceipt/manifest unionをexhaustiveに追加し、新profile全件fixture、両OSのhost evidence、原ノ味全巻ゲートが揃うまで登録はprivate stagingに留める。rollbackは新profile受付を削除する小変更ではなく、公開change set全体を戻し、既存1.4 pipelineを残す。

調査中の既存回帰確認:

```sh
cargo test --manifest-path workspace/Cargo.toml \
  --package typaxis-resource-admission safe_svg_2 --locked
```

結果: 11 passed / 0 failed。これは既存テストが今回の不整合を検出していなかったことを示すbaselineであり、新設計の実装成功ではない。設計書の相対リンクと差分の空白も検査した。

## 10. 完了判定

必須ゲートは、VMBの対象全巻を、対応日本語フォントを明示した**一つのDocumentPackage**として`check-package`と`build-package`で処理し、全章を含む一つのPDFを生成することである。章分割、画像・数式の省略、ラスタ化fallbackを合格扱いにしない。

そのうえで、5,000画像入力の既定設定での成功、raw数式SVGの複数パス受理、正確な診断、数式のbaseline/spacing、描画/抽出/タグの独立検証、同内容Form共有、旧経路回帰、決定性を満たすことを要求する。実書籍の画像/数式数がexporter修正で変わった場合は元ノードとの対応表で説明し、件数だけを都合よく更新しない。

原ノ味明朝は、§7.3を完了し実フォントの同じSHA-256で全巻ゲートを通すまでは正式対応に含めない。未対応期間も、`VORG`/cmap/CID等の構造上の理由と埋め込み権限を区別した診断を必須とする。

## 11. 外部一次資料

- [W3C SVG 2 Paths](https://www.w3.org/TR/SVG2/paths.html): path/subpath/commandの意味と一般SVG grammar。Typaxisの閉じたsubsetとの比較に使用。
- [原ノ味フォント公式README](https://github.com/trueroad/HaranoAjiFonts/blob/master/README.en.md): Adobe-Japan1 CIDへの変換により作られるフォントであることを確認。今回の具体的なtable・fsType・FD数はVMB同梱bytesのローカル解析による。
- [OpenType cmap](https://learn.microsoft.com/en-us/typography/opentype/spec/cmap): format 14とbase Unicode cmapの関係。
- [OpenType VORG](https://learn.microsoft.com/en-us/typography/opentype/spec/vorg): 縦原点tableの構造。水平baselineとは区別する。
- [Adobe CFF仕様](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf): CID charset、FDArray/FDSelect、Private DICTの関係。

## 12. 詳細化レビュー記録

| review | finding | 対応 |
| --- | --- | --- |
| 1 | VMBの正式exporterが存在する前提だった | 新設ownerをVMB docsへ定義し、既存TSF中心案との移行を明記 |
| 1 | OriginXの意味・Paddingを含むmetricsの差を扱っていなかった | engine writerとplacementの実装を照合し、逆符号とPadding込みの変換へ修正 |
| 1 | 詳細エラーが現在のCopy API/CLIへ届く方法が未定義 | fixed-size context、detailed API、typed code mappingとnotesを定義 |
| 1 | config適用後のdefault上書き・carrier pointer転写の危険 | merge前profile defaults、元arrayのcount検査と元pointerの保持を定義 |
| 2 | CID対応がwhitelist拡張の列挙に留まっていた | FD別program、評価context、UVS coverage、subset/PDF接続と1.5公開境界を固定 |
| 2 | inspectionコマンド名と拡張範囲が不正確 | 実CLIの`inspect-font FONT`を確認し、既存JSON shapeを保持 |
| 2 | 性能修正、5,000 distinct生成、描画許容差が未確定 | cursor/index、固定数式template、oracle/tamper試験・mask閾値を定義 |
| 3 | VMB側とのActualText/face index入力仕様の曖昧さ | semantic speechからの明示ActualText、TTCの選択flagと未指定規則をVMB docsで固定 |

最終レビューでは両文書の責務、profile/contract識別子、budget、単位、origin/baseline、provenance、fixture/runner、公開順を再照合した。相対リンク、JSON/TOML例、Markdown fenceの検査、および6桁小数→16.16の131,073個のresidue往復検証が成功した。11ptのorigin/Padding例も確認した。未解決の設計findingはない。

製品実装・新規runnerの実行・原ノ味版/TrueType版の全巻PDF生成は未実施。§10とVMB側設計の入力・実装ゲートを、文書レビュー完了と取り違えない。
