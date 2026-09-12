# VMB全巻PDFの互換性・リソース予算・診断改善設計

状態: Implementing（[実装・検証台帳](28-vmb-book-production-progress.md)を参照。全巻PDFの成功確認は未実施）
調査日: 2026-09-05
Typaxis baseline: `718ab6c9e1309b7dc750c62554c954cae4333131`
対象: `typaxis.contract/1.4` / `typaxis.machine-pdf/production-book-1`

詳細化: 2026-09-05。設計照合: 2026-09-06。本書はTypaxis側の正本。VMB側の変更はユーザー指定の[VMB docs/typaxis-book-export-design.md](../../../v/vmb-container/docs/typaxis-book-export-design.md)を正本とし、処理境界・共通受け入れ条件のみ本書にも記載する。「新設」「提案API」は設計時点の変更案であり、その後の実装有無は実装・検証台帳で確認する。「次期」の公開識別子は、公開ゲートを通すまで使用可能とは扱わない。

依頼された4項目への対応は次のとおり。§2の「現行」は冒頭のbaselineでの調査結果を指し、現在の実装状況とは区別する。先行実装・実行結果・残件は[実装・検証台帳](28-vmb-book-production-progress.md)を参照する。

| 要望 | 具体的な修正と担当 | 合格条件・詳細 |
| --- | --- | --- |
| 1. 複数パスの数式SVG | TypaxisのSafe-SVG 2 scannerでタグ終端前の空白を受理し、独立pathの描画順と同一pathのsubpathを保持する。失敗原因をresource-localな型で保持してCLIへ伝える。VMBは内部単位を物理ptへ変換し、root寸法と配置metricsを同時に確定する。 | group/root直下の複数path、M/L/C/Q/Z、小数、同じ縦横比のpt寸法、実分布のpath/segment数を検査し、独立描画比較で欠落・切断・穴の変化がないことを確認する。§4〜5、§8。 |
| 2. 画像数上限 | production-book-1の未指定値を8,192画像、262,144 vector nodes、4,000,000処理segments、depth 32へ変更する。既存config・環境・CLIの明示値を優先し、hash共有を維持する。capabilitiesの詳細上限は次期schemaで公開する。 | 5,000 distinct画像を実配置してcheck/buildが成功する。8,192/8,193と明示1,024/1,025の境界、共有Form数と配置数、時間・RSSを別々に検証する。§6、§8。 |
| 3. 和文フォント | 先行修正はtable・phase・offset・埋め込み権限・TTC face情報の診断。原ノ味正式対応はVORG等のtable、cmap 14、CID/FD別CFF解析・評価・subset・PDF埋め込みを新profileとして実装する。 | 現物のfsTypeは0で、最初の未対応tableはVORG。TrueType全巻ゲートとは別に、同じ原ノ味ファイルのhashで全巻・日本語抽出・IVS・subset後描画を検証する。§7、§9.1。 |
| 4. VMB結合テスト | 実engine出力と元書籍fixtureを由来付きで保存し、小規模→300〜500数式の章→5,000画像→実全巻のrunnerを作る。Typaxisの本文と数式は共通の行・ページ配置へ接続し、VMBはsource projectionと意味情報を正しく生成する。 | 同一package/configでcheck/buildし、一巻一PDF、描画・baseline/spacing・抽出順・Formula/Alt/ActualText・タグ・リンクを独立検証する。§8、§10、§14。 |

設計の受け入れ条件は全巻PDFの生成・内容検証までである。parserや予算の単体試験、未配置画像のcheck成功、小規模PDFの生成だけでは完了としない。本文・数式の共通組版（§14）とVMB exporterの正式接続も、当初の全巻要件を満たすための必須修正に含める。

今回の設計確認（2026-09-06）では、現在のコードと保存済み再現結果を照合し、元全巻packageのSHA-256・4,514画像、原ノ味ファイルのSHA-256・`fsType=0`・cmap 4/12/14を再確認した。Safe-SVG 2の終端前空白受理とproduction-book-1のprofile別予算値は既にコードへ反映されている。したがって今後の実装では、これらを再実装するのではなく、未完の詳細診断と実入力・公開check/buildの受け入れ試験を完結させる。フォントのadmission・container/face・CLI詳細診断は今回接続したが、charstring/subset等の詳細化は残る（実装台帳の同日追補参照）。原ノ味正式対応、正式VMB exporter、5,000 distinct画像と実全巻PDFの成功は未確認である。以下の設計仕様と、台帳に記した実装・検証状況を分けて読むこと。

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

実装追補（2026-09-06）: VMBに`MathExportSession`を追加し、Prepare済み入力を既存の実math adapterへ渡して、上記の寸法変換・配置別TeX/speech・内容hash共有へ接続した。明示式番号の子source span、配置ごとのspacingと一括wire照合も保持する。VMB設計§15.22と実装台帳に実行結果を記録した。公開checkで2画像・inline/block各1配置が受理されたが、既知の試験用package外枠を使った部品の検証であり、正式RenderBook全走査・PNG/JPEGとの共通資源列・公開buildと全巻PDFは未完了である。

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

| limit | 調査baseline | production-book-1の採用値 | 課金単位 |
| --- | ---: | ---: | --- |
| `max_images` | 1,024 | 8,192 | logical画像宣言数。未使用・aliasも含む |
| `max_vector_nodes` | 100,000 | 262,144 | 文書内の各vector宣言に対する解析node合計 |
| `max_vector_path_segments` | 1,000,000 | 4,000,000 | stored segments + 外周clip + clip replayの合計 |
| `max_vector_nesting_depth` | 32 | 32 | resource内深さ |

2026-09-06のコード照合では、`MachineResourceDefaults::for_profile`とCLIのprofile別config resolverに上記の採用値が存在する。表の左列は調査当初の値であり、現checkoutの既定値ではない。この実装確認だけで、5,000 distinct画像のbuildや全巻PDFが成功したとは判定しない。

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

新しい`evaluate_glyph_v2`は最初にfd_by_gid[gid]を取り、評価終了までFDを保持する。`ProgramKindV2::Local { fd, index }`と`Global { index }`を区別し、global→local callでもそのglyphのFDを利用する。widthは選択FDのdefault/nominalで解決してCFF source widthとして保持する。OpenTypeの組版advanceはhmtxを使用し、source CFF widthとの一致をadmission条件にしない（§7.7の実測・仕様訂正）。subsetのCFF幅・hmtx・PDF Widthsは選択されたhmtx advanceから一貫して生成する。operator/stack/call-depth/stem/operation/outline予算はV1と同じ上限を継承する。

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

### 7.6 実装追補: CID CFF /2 programの構造検査

`typaxis-font/src/cff_v2.rs`の`inspect_cff1_program_v2`は元CFF tableを
`Arc<[u8]>`で一つ保持し、CharStrings・global/local Subrsは検証済みbyte rangeで参照する。
Top/FD/Private DICTを区別し、ROS・CID charset 0/1/2・FDSelect 0/3・FDArray 1〜256を
検査する。DICT operandは49個目の割当て前に拒否し、INDEX countもoffset表の割当て前に
上限を検査する。global数と全FDのlocal数を一つのsubroutine上限へ加算する。

完全に同一のPrivate DICTまたはlocal INDEXをFD間で共有することは認めるが、宣言ごとの
課金は省略しない。構造の部分重複や、異なる種類の構造を同一範囲へ置く入力は拒否する。
GID 0のCIDは0とし、残りのCIDの重複・範囲overflowを拒否する。CIDはSIDやUnicodeとして
解釈しない。エラーはCFF table内offset・FD・operator・limit/observedを保持する。

無変更の原ノ味明朝について、23,060 glyph、18 FD、1,600 globalと24,956 local Subrsを
確認した。実際にglyphが参照するFDは12種類であり、未使用FDも構造検査と宣言数に含める。
全GIDのFD/CID列は`tools/inspect_harano_cff_program.py`のFontToolsによる独立読取りと
hash一致した。構造規則の参照元は[Adobe CFF仕様 §18–19](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5176.CFF.pdf)。

この入口はCFF programの構造inspectionであり、sfnt admission、embedding permission、
Type2実行、cmap/IVS、subset/PDF認可を発行しない。既存CFF `/1`を変更せず、公開の
contract 1.5 / production-book-2 / resource profile `/2`の有効化は本設計の後続ゲートまで行わない。

### 7.7 FDを固定したType2実行とOpenType幅の訂正

`CffProgramEvaluationSessionV2`は構造検査済みprogramのGIDからFDを一度選択し、
local呼出しを`Local { fd, index }`として解決する。global→localでもglyphのFDは変えない。
operand/call-depth/stem/mask/outlineの実行機構は既存評価器と共有するが、CFF `/1`の
既存方針は維持する。`/2`はsubroutine中のendcharによるglyph終了を認める。
これは[Adobe Type2仕様 §4.2 Note 6](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5177.Type2.pdf)
に従い、元原ノ味の.notdefにも実際に使われる。

本実装時に、元原ノ味の310 glyphでCFF source widthとhmtx advanceが異なることを確認した。
例はGID 151で346対1000、GID 233で1000対500である。
[OpenType hmtx仕様](https://learn.microsoft.com/en-us/typography/opentype/spec/hmtx)では、
CFF内の幅はPostScript向けであり、OpenType組版はhmtxを使う。したがって本設計の
「hmtxとの整合」は元の二値の一致を要求するものではなく、admitted hmtxをshaping・
subset hmtx・subset CFF幅・PDF Widthsへ一貫して伝えることとする。
CFF幅のoperand/default/nominalはchecked 16.16演算で解決し、別のsource widthとして保持する。
fontを編集したり、文字のadvanceをCFF幅へ置き換えたりしない。

無変更の原ノ味23,060 glyphを一つの既定budgetで実行し、8,376,159 operations・
1,572,638 outline segmentsで成功した。FontToolsによる独立実行と全輪郭のcommand/座標列、
全CFF幅のhashが一致する。これは全glyph実行を用いた検証であり、最終admissionの通常経路が
全glyphを事前実行するという意味ではない。通常は選択GIDだけを実行する。

このsessionはまだinspection ownerである。admitted source/face/profileへ結んだcache・
selected-glyph closure・subset receipt、sfnt/cmap14/IVS、公開PDFと全巻ゲートは引き続き必要。
既存CFF `/1`のprogram型とresource/manifest identity、および公開profile registryは変更しない。

### 7.8 縦tableと異体字対応表の構造検査

`validate_cff_vertical_metrics_v2`は元tableを借用し、VORGの疎な上書きと
vmtxの末尾short bearingを展開せずに取得する。vhea versionはOpenType仕様の
`0x00010000`と`0x00011000`、予約fieldとmetricDataFormatは0を要求する。
version 1.0のlineGapも予約値0として検査する。水平組版には適用しない。
構造検査はvheaの集計extremaと全glyphの値の一致を追加条件としない。

`validate_cff_variation_sequences_v2`はcmapのencoding recordからplatform 0 /
encoding 5のformat 14を一つだけ取得する。selectorと各Unicode範囲の順序・重複・
scalar範囲、GID範囲、default/non-default間の交差とpayloadの部分重複を拒否する。
同じ種類のpayloadの完全共有は認めるが、値数はselectorごとに課金する。
selector上限256、展開相当値数上限1,000,000はpayload走査前および圧縮rangeの加算時に
検査する。lookupは圧縮tableを借用し、Default / NonDefault(GID) / Missingを区別する。
基底cmapの検証と、二scalarを保持したIVS shapingは別の必須工程である。

原ノ味の全23,060縦metricと17 selector・14,780対応値を、元fontのhashを固定して
FontToolsの独立取得と照合した。再現用toolは`tools/inspect_harano_cff_tables.py`。
ここまでで正式sfnt admission、subset、公開PDF、IVS抽出の対応完了とは扱わない。

### 7.9 CID CFF /2のsfnt admissionと基底/異体字coverage

`admit_sfnt_cff1_v2`は無変更のsfnt bytesを所有する`Cff1AdmissionV2`を返す。
face 0、source bytes予算、directoryの順序・範囲・checksum、必須table、既存の任意table、
OS/2埋め込み権限、縦table、基底/補助cmap、CID programを同じ入力へ接続する。
CFF Name INDEXとsfnt PostScript名、Top FontBBoxとhead bboxも一回のprogram検査内で
照合する。glyph数とhmtxはそのadmissionが保持し、source hash・face・実効予算のfingerprint・
`typaxis.sfnt-cff1-admission/2`・resource profile `/2`をidentityへ結び付ける。
この経路では未選択glyphのType2実行を行わない。

`CffCmapV2`は既存format 4/12の検証・整合性照合で作る基底mapと、検証済みformat 14を
所有する。format 14を基底mapへ混ぜない。`glyph_for_sequence`はdefaultなら基底map、
non-defaultなら指定GIDを返し、未対応pair・孤立selector・欠けたdefault基底・GID 0は
coverageなしとする。この問い合わせは文字置換ではなく、元二scalarをshaperへ渡す前の検査に使う。

原ノ味の全sfnt admission、15,815基底対応と14,780異体字の解決GIDを独立FontTools結果と
照合した。権限制限とVORG/vhea破損はchecksumを修正した負例で検査し、元fontの成功とは
区別する。公開CLIのresource registryはまだこのownerへ接続していない。旧 `/1` は
従来どおりVORGをunsupportedとして拒否する。

選択GIDのcache/closure/subset、IVSの実shaping/source cluster/ToUnicode、公開manifestと
全巻ゲートは必須残件である。name-keyed入力の `/2` 共通経路への接続も残る。
この内部admissionの成功だけではcontract 1.5 / production-book-2を有効にしない。

### 7.10 選択closureとsourceを固定した評価cache

`Cff1SubsetSessionV2::close_instance_selection`は全選択GIDの範囲とinstanceのCID上限を
評価前に検査する。既存 `/1` と同じく上限は非zero GID数へ適用し、`.notdef`を一度だけ
先頭へ加える。残りはsource GID昇順で、`Cff1GlyphClosureV2::subset_gid`はその順位を
dense subset GIDとして返す。closureはadmission fingerprint、source hash、face ID、
instance ID、選択順序を `/2` identityへ結ぶ。

sessionはadmissionと同じ実効予算だけを受け付け、元hmtxのadvanceで選択glyphを評価する。
現在のownerはprofile `/2`、standalone face index 0に固定されるため、cacheの可変keyは
source SHA-256とGIDである。異なるinstance/face IDのaliasでも同じsource/GIDは一回だけ
課金し、異なるsourceは輪郭が同じでも共有しない。失敗までに消費したworkは維持し、
成功したglyphだけをcacheへ格納する。

`prepare_face`は渡された全GIDを先に検査してから`.notdef`と昇順GIDを実行する。
複数instanceを扱う上位ownerは、全instanceのclosureを先に作り、faceごとのunionを
FontFaceId順に準備する必要がある。現時点のsessionは選択評価を所有するが、subset bytesの
生成とreceipt、上位resource ownerへの接続、公開PDFはまだ実装完了とは扱わない。

### 7.11 実OpenType subsetと独立輪郭・描画検証

`Cff1SubsetSessionV2::subset`はsealed closureを検証・準備した後、選択された輪郭から
dense CIDのCFFを生成する。既存のcanonical Type2 serializer、sfnt checksum、name/head、
水平metricとPDF metricのrecipeを共有し、旧 `/1` の出力を変更しない。
生成CFFの幅、subset hmtx、`Cff1SubsetV2::original_widths`はadmitted hmtxへ一致させる。
生成bytesのSHA-256、closure fingerprint、source hash、subset名を `/2` receiptへ結ぶ。
CharString bytesの小計と最終sfnt長にsubset byte上限を適用し、最終sfnt bufferは長さを
確認してから作る。一時serializer allocationを含む全stageの累積課金は引き続き別の必須工程である。

subset cmapは選択された基底mapとUVSを保持する。UVSはdefault/non-defaultのどちらも、
解決済みのdense GIDを持つ明示non-default mappingへcanonical化する。基底mapのない
non-default専用GIDのsubsetも扱い、format 12は空にできるが、/2入力の空基底mapには
実際に使用可能なnon-default UVSを要求する。format 14だけで基底tableを省略することは
認めず、未対応pairを基底文字へ落とさない。

元原ノ味から28 glyph（実使用の全12 FD、和文・句読点・幅が異なる2 glyph、UVSを含む）を
選択して5,052-byte subsetを生成した。FontToolsは全選択輪郭、元hmtxのadvance/bearing、
生成CFF幅、dense CID、27基底対応と12 UVSを照合する。FreeType 2.14.3では全glyphを
12/24/48/96pxで描画し、112件のbitmap・位置・advanceが一致した。
canonical subsetは元hint programを保持しないため、描画比較は双方で
[hintingとbitmap strikeを無効](https://freetype.org/freetype2/docs/reference/ft2-glyph_retrieval.html)にする。

元font用admissionは従来どおりsubset prefix名を入力拒否するため、生成fontを元fontとして
再admitすることを検証の代用にしない。生成物はsfnt構造検査と上記の独立parse/renderで検査する。
5,052 bytesで成功し、5,051 bytesで出力を返さない境界も検証した。
これは選択subsetの検証であり、IVSの実shaping/cluster/ToUnicode、public resource・PDF・
manifestへの接続、Harano全巻・scaleの独立検査は引き続き必須残件である。

### 7.12 元二scalarの実シェーピングと全UVS subset join

`shape_cff1_run_v2`はsealed `Cff1AdmissionV2`の元bytes・face 0・metric・実効予算を
既存の`shape_linked`へ渡すfont単位の接続である。入力UTF-8とpre/post contextのbytesを
予算検査し、harfrustの既存record上限・事前予約・実出力課金を共有する。
基底文字の直後にVSがあれば必ずpair coverageを検査する。孤立VS、Missing pair、run外の
post contextへ分割されたVSを拒否し、VSを削除・置換して再試行しない。
旧 `/1` のdefault-ignorable規則は変更しない。

harfrustへは元二scalarをそのままinputとして渡す。`Cff1ShapedRunV2`はadmissionと元UTF-8を
保持し、`cluster_text`は実clusterの元文字列を返す。複数sequenceが同じGIDへ対応しても
文字列をGIDから推測しない。source span長とUTF-8 byte境界も既存backendで検証する。
このownerはpackageのstyle/font選択、layout epochやPDF publicationを認可するものではなく、
その上位ownerへの接続は引き続き必要である。

元原ノ味の全14,780 UVSを200 pairごとのrunで実シェーピングし、全GID・全source clusterと
元文字列を照合した。実shape出力の選択集合から14,674 glyphのsubsetを生成し、全UVSが
dense GIDへ正しくjoinする。FontToolsは全選択輪郭、14,002基底対応と全14,780 UVSを照合し、
FreeTypeでは58,696件のunhinted raster・位置・advance比較が一致した。
既定CFF予算で7,422,048 operations・1,354,039 outline segmentsを使用し、subsetは
6,563,684 bytesとなる。全UVSが使うFDは4種類であり、§7.11の全実使用12 FD試験とは区別する。

この全UVS検証は公開PDFのToUnicode/ActualText抽出やHarano全巻ゲートを代替しない。
§9.1に従うprivate stagingのprofile別resource/shaping/PDF/manifest union接続と、
正式exporter・共通layout・公開CLI・両OSの全巻検証は必須残件である。

### 7.13 CFF /2のresource planとPDF font object接続

`freeze_cff1_pdf_fonts_v2`はsealed admissionと実`Cff1ShapedRunV2`を受け取り、
全instanceの選択集合を閉じてからface/GID順のunionを単一CFF sessionで評価する。
結果は旧`FrozenPdfFontPlan`から独立した`FrozenPdfCff1PlanV2`となる。
異なるadmission・実効予算・instance、重複run/instanceを拒否する。
文字サイズはclusterごとに保持し、同一faceの異なるサイズでもsubsetを共有する。

planはsubset、dense widths、CID bindings、clusterの元文字列・出典・ActualText要否を
保持する。Unicode候補が競合するGIDや複数scalar/glyphのclusterを単一scalarへ潰さない。
CIDのToUnicodeを連結して元文字列と一致しないclusterではActualTextを要求する。
識別子はparsed/generated namespace、buffer ID・byte範囲に加え、生成文のowner・
generation kind・owner-local ordinalも含む。canonical JSONはescape後の保守的上限を
spool予算へ事前課金する。この局所予算は文書全工程の累積allocation予算を代替しない。

`encode_cff1_pdf_objects_v2`はこのplanを直接消費し、Type0、CIDFontType0、descriptor、
FontFile3/OpenType、ToUnicode、CIDSetの6 objectを生成する。旧receiptへのcastは行わない。
旧profileと共通のToUnicode/CIDSet serializerを使用し、object範囲と実効予算の同一性を
検証する。戻り値は相対offsetを持つfont contributionであり、文書assemblerのobject予約・
衝突検査・paint/terminal/manifest照合や`VerifiedPdfBytesReceipt`を認可しない。

元原ノ味の基底文字と同一GIDへ対応するIVSを実shapeし、font contributionを診断用1 pageへ
組み込んだ。Poppler 26.08.0とMuPDF 1.28.2の双方が`A一一󠄀日本語`を二scalarのIVSも
含めて正確に抽出した。Popplerは埋め込みfontを`CID Type 0C (OT)`と認識する。
元文字列が同じでも出典bufferが変わればplan fingerprintが変わり、subset bytesは同じである
ことも検証した。生成文のbuffer ID・範囲が同じでも生成owner/kindは区別する。

これはfont単位のprivate-staging接続である。公開profile/manifest union、packageのfont選択、
共通layoutからのpaint照合、正式exporter、全巻・scale・両OSの検証は引き続き必須である。

### 7.14 resource-set /3のhost admissionとfont instance

`StagingProductionResourceResolverV3`は既存のhost root・stable read session・累積resource bytes・
vector work budgetを共有し、CFF /2を別mapで保持する。宣言media、face index、URI、期待hashを
検証したうえでsealed `Cff1AdmissionV2`を生成する。CFF詳細エラーはfont face IDと
`Cff1FailureV2`のまま保持する。別sessionのpending bytesや重複bindingは拒否する。

確定先は新しい`AdmittedProductionResourceLedgerV3`だけである。font variantはTrueTypeと
CFF /2を明示的に区別し、CFF /1をこのledgerへ入れる経路や、CFF /2を旧ledgerへcastする
経路は提供しない。画像の実admission、declaration順のdense ID、vector digest alias検査、
family tableを共有する。fingerprintのdomainは`typaxis.admitted-production-resources/3`で、
resource-set /3、元bytes・宣言、CFF admission、実効予算、上位profile fingerprintを束縛する。
canonical JSONはescapeを考慮した保守的容量をspool上限内で事前予約する。

`AdmittedProductionFontInstancesV3`は確定したledgerを借用し、選択faceの順序・重複によらず
dense instance IDを発行する。解決先のledgerを呼出側から差し替える引数は持たない。
これはfont選択後のinstance所有者であり、style選択、line reshaping、paintやmanifestの
上位ownerへの接続を代替しない。

実host読み込みでTrueType、原ノ味の同一bytesを持つ2宣言、PNGを混在させた。異なる読み込み順で
fingerprintが一致し、各CFF /2 variantに元23,060 glyphと元bytesを保持する。別のadmission
sessionは同じfingerprintでもruntime sessionとして区別する。旧resolverでは同じ原ノ味を拒否する。
通常fixtureではfont+imageの合計読み込みbytesの上限ちょうどで確定し、1 byte小さい上限では
image readを繰り返しても失敗し、ledgerは確定しない。

このAPIはprivate stagingであり、上位のcontract 1.5 / production-book-2 preflight receipt、
profile・manifestのexhaustive union、共通layoutへの接続、公開dispatchは残件である。
既存CLIのprofile登録やschema aliasは変更しない。

### 7.15 ledger由来のinstanceから実shapeとCFF計画への接続

`shape_production_run_v3`はledgerが発行したinstanceからfont bytes・face index・metadataを
導出し、TrueTypeとCFF /2を明示分岐して既存linked backendでshapeする。呼出側が別のfont IDや
bytesを差し込む引数は持たない。出力は実instanceと元request（source、UTF-8、サイズ、
language/script、bidi level、pre/post context）を保持する。featureは既存backendの固定defaultである。

instance tableは選択face集合とledger fingerprintを`typaxis.production-font-instances/3`で
識別する。別の選択集合で同じ数値instance IDを再利用しても、同じtableとは扱わない。
`freeze_production_cff1_fonts_v3`は実shape出力からadmission・face IDを導出し、同一runtime
session・ledger fingerprint・instance tableであることを検査する。異なるsessionやtableの
runを混ぜず、CFF以外のrunを黙って除外しない。確定したCFF集合も元ledgerへの参照を保持する。

元原ノ味の11pt IVS付き本文と22pt基底文字の同一GIDが、1 subset・1 CIDを共有することを
実host read→instance→shape→subsetで検証した。advanceは実shapeで2倍となり、各clusterは
異なるfont sizeと元文字列を保持する。font単位の単一size制約は§14.3の共有方針に反するため
除去した。source/sizeを含むplan fingerprintは維持する。TrueTypeの実shape、source長不一致と
context超過の拒否も検証した。これは上位のstyle選択・行再shape・page/paint/manifest閉包や、
TrueType/CFF混在文書全体のfont finalizer・公開PDFの完成を意味しない。

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

### 8.7 本文と数式を混在させたPDFの合格条件

抽出期待値はVMBの本文bytes・配置順・semantic speechから、PDF生成前に作る。最小fixtureを「本文A＋空白＋inline式＋本文B」とし、式の後に空白を持つ別caseも作る。和文隣接、連続数式、block式、改ページも別caseにする。式の前後へ空白を追加・削除した出力は、数式のspeechが合っていても不合格とする。PopplerとMuPDFの抽出結果をそれぞれ照合し、一方だけの成功を双方の成功として記録しない。

本文の視覚検査には、実際に非空の輪郭を持つ配布可能な固定フォントを使う。空輪郭の合成fontはCID・ToUnicode等の構造単体試験には使えるが、本文が表示されることや本文と数式の視覚的な間隔を証明するfixtureには使わない。本文glyphと数式の双方について、独立renderで非空のinkと選択済みbaselineを照合する。

FormulaタグとAlt/ActualTextの存在だけでは抽出成功とは判定しない。SVG Form自体に文字描画がない場合も、各配置の意味テキストが一回だけ、正しい順序・位置で抽出されるPDF出力方法をTypaxisのPDF層で検証する。抽出互換性のために補助要素が必要な場合は、選択済みbaselineとviewportに束縛し、描画比較で追加inkがないこと、余分な抽出文字・MCID・source occurrenceを作らないこと、object/spool予算へ課金することを要求する。VMB側に不可視文字やPDF用glyphを出力させない。

本文のActualTextは、MCIDを共有する選択済みsource owner・行fragmentの文字列から構成する。元source nodeの全文を各行へ繰り返さず、複数clusterのreplacementを入れ子にしない。本文encoderが保持するglyph描画commandの型付き範囲を使い、最後の本文font/matrixが有効な間にActualTextを閉じ、その後にgraphics stateを復元する。生成済みPDFへの文字列置換や、抽出器の空白推定に合わせた描画位置の変更で代替しない。

途中段階のPDF組立てAPIから得た検査用bytesと、公開`build-package`が発行する検証済み成果物を区別する。ページ・構造・navigation・font・描画の整合検査と既存の成果物検証を経て、PDFとmanifestを同時に公開するまで、§10の全巻成功やPDF/UA適合を宣言しない。

## 9. 実装順序と変更owner

| 順序 | 作業 | 主なowner / 完了条件 |
| --- | --- | --- |
| 1 | 実データfixture固定・仕様訂正ADR | 新corpus、ADR-0038、docs/27追補。今回の再現と元hashを保存 |
| 2 | エラー詳細保持・carrier error伝播 | resource-admission、document-package、diagnostics、CLI。原位置・原因がJSON notesとstderrへ到達 |
| 3 | V2タグ終端空白 | safe_vector scanner policy。raw実章SVG成功、旧V1/V2受理入力のgolden不変 |
| 4 | profile defaultsと大規模予算 | coreのprofile default owner、CLI config resolver、decode/admission。override優先・5,000件試験 |
| 5 | VMB exporterの寸法/単位/意味情報 | VMB Typaxis backend。全巻SVG・metrics・altを同じ入力から再生成 |
| 5a | 本文・数式の共通組版と本文fontの独立化 | §14。空native-math chain、実glyph advance、本文と式の同一flow、実placement由来のPDF/タグ/リンク |
| 6 | 実章・5,000件・TrueType全巻ゲート | CLI E2E、Python verifier、管理ホスト。単一packageの検証・PDFと独立検査が成功 |
| 7 | 原ノ味正式対応 | 新CFF profile/次期contract ADR→font/admission/shaping/resources/PDF/manifest→原ノ味全巻ゲート |
| 8 | capabilities拡張 | 次期registry公開時にSchema/encoder/golden/guideを一括更新 |

2・3・4はそれぞれfocused regressionを通し、6の前に結合する。5・5aが未完了なら全巻成功を主張しない。7・8を先行する小修正へ混ぜない。

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

初回レビューでは両文書の責務、profile/contract識別子、budget、単位、origin/baseline、provenance、fixture/runner、公開順を再照合した。相対リンク、JSON/TOML例、Markdown fenceの検査、および6桁小数→16.16の131,073個のresidue往復検証が成功した。11ptのorigin/Padding例も確認した。この時点では設計findingなしとしたが、その後の実build調査で§14の不足が判明した。

初回レビュー完了時点では製品実装・新規runnerの実行・原ノ味版/TrueType版の全巻PDF生成は未実施だった。その後の先行修正と試験結果は[実装記録](28-vmb-book-production-progress.md)を参照する。§10とVMB側設計の入力・実装ゲートを、文書レビュー完了と取り違えない。


## 13. 実装中の診断表現補足

既存`CanonicalDiagnosticText`はraw input snippetを拒否する。既存Schemaを変えずに位置・tokenの原因情報を残すため、resource-local noteのtokenはUTF-8境界で80 bytesまで保持し、`token_percent=%31%30...`のpercent encoding（最大240 ASCII bytes）で投影する。復号可能な元tokenと`token_truncated`を保持し、SVG自体のbyte span/line/byte-columnは別表示する。閉じたSVG element/attribute名はそのまま表示し、未知名もpercent encodingを使う。この変更はraw excerptの無制限出力や診断文字列の一般的な制約緩和を許可しない。


実CLI照合による補足: production-book-1のbuildは非圧縮設定を要求する。check/buildの共通TOMLに`pdf_stream_compression = "none"`を明示し、両方に`--config PATH`を渡す。checkには`--no-compress`がない。診断出力は`--emit-diagnostics PATH`、build manifestは`--emit-build-manifest PATH`を使う。画像予算以外のこの既存profile条件は本修正で変更しない。


実章probeでは、元SVGのまま`check-package`が成功し、共通非圧縮設定での`build-package`は`L5100: semantic_container is not allowed in this owner`で停止した。後続調査で、章にはsemantic_containerが存在せず、native math必須条件のエラー表示であると判明した。本文PDF経路にも同じ依存があり、§14で一体として修正する。全巻font-only probeの`semantic source span ownership mismatch`は、最初の数式node 7（`/document/blocks/2/children/1`）のspan `0..1`を親paragraph node 5のspan `0..0`が包含しないことが原因。VMB側のsource projection設計で直す。

## 14. 実buildで判明した本文・数式の統合不足

### 14.1 確認した事実と修正範囲

`typaxis-cli/src/pipeline.rs::build_production_book_pdf`はnative math authorizationを無条件に作る。一方、`StagingMathProfileView::new_with_mode`はnative `math_nodes`が空ならInvalidNestingを返す。SVG数式はこの集合に入らない。production用だけ空集合のsealed chainを認め、旧math-only sliceの非空条件は維持する。空集合でもpackage/limits/profile/admission/sessionの照合を省略しない。実装時の小規模試験ではこの変更でcheckとauthorizationは成功したが、buildは次の本文font依存で`I9190: production tagged-PDF native math mismatch`へ進んだ。これを完成した修正と扱わない。

`typaxis-pdf/src/tagged_pdf_v2.rs`は本文を最初のnative math fontで描く。`emit_native_math_font_objects_v2`と`encode_standard_text_paint_v2`がその集合の先頭を要求し、`MathFontFace::parse`はMATH・glyf・locaを必須にする。本文TrueTypeにMATHを要求してはならず、CFF本文にもこの経路を使ってはならない。ダミーnative数式の挿入やMATHテーブルの追加では直さない。

さらに現行production bridgeは、インライン前後の文字を10pt固定advance、8pt/2pt ascent/descent、20pt行高として扱い、本文描画位置と2〜8ptのfont sizeをページ内record数から再計算する。通常本文のpageは多くの場合0、リンク位置も仮の等間隔座標である。これでは式のbaseline/前後空白、全巻の読み順・改ページ・本文スタイルを保証できない。旧combined fixtureのPDF成功は、この書籍の受け入れ証拠にはならない。

### 14.2 共通flowと選択済み配置の設計

新しいproduction専用bridgeを`typaxis-syntax`、`typaxis-shaping`、`typaxis-layout`、`typaxis-pagination`、`typaxis-display-list`に設ける。既存のfrozen staging fixture helperは回帰比較用に残し、production runnerから固定metrics helperと仮座標を使わなくする。

1. syntaxが検証済みwire、computed style、text mappingから、本文・見出し・リスト・caption・表cell・脚注の順序付きtext siteと数式placeholderを発行する。siteはsource owner、style owner、TextSpan、exact UTF-8、effective language、package fingerprintを持つ。raw JSONからPDFへ直接文字列を渡さない。
2. shapingは既存のadmitted font family解決とharfrust経路を共有する。本文は通常OpenTypeのglyph/cluster/advance/offsetを使い、MATHはnative数式を組む場合だけ参照する。Unicode・言語・direction・font size・feature設定をreceiptへ結び、未知glyphやfont selection failureは入力node付き診断にする。本文fontを画像宣言順や最初のnative数式から選ばない。
3. paragraph item列へshaped text cluster、atomic SVG数式、native数式、break/spacingを元順序で入れる。本文だけの段落も列へ入れる。SVGのadvance/ink bounds/origin/baselineと前後spacingは§5のmetricsを使い、spacingを文字のadvanceへ重複加算しない。native数式も同じ行・block cursorを消費する。
4. 一つのflow cursorで本文、見出し、display math、図・caption、リスト、table、forced break、脚注を順にpaginateする。既存のlinebreak・table・footnote・keep・overflow規則を用い、別々にページ0から配置した結果を後で重ねない。固定20ptのcaption/keep successor見積りと、inlineがあるとbody高をほぼ消費済みにする現在の仮入力を廃止する。
5. private fieldの選択済みreceiptを発行する。paragraph/line/fragment、source owner、style、font/glyph cluster、page/frame、baseline/rect、paint ordinal、exact text/ActualText、admitted/profile/limits/session identityを結ぶ。同じ本文fragmentとSVG配置をdisplay・structure・navigationへ投影する。
6. structureのMCRは実際にpaintしたfragmentから生成する。改行/改ページで複数fragmentになる本文には異なるsemantic fragment ordinalを付ける。outline destinationは該当headingの最初の配置、link annotationは実際のlink glyph範囲から作り、複数行リンクは行ごとの矩形にする。page 0や固定矩形をfallbackにしない。

新receiptのcanonical identityはlayout/displayの実装変更と一緒に更新し、旧recipeのfingerprintを新結果へ転用しない。1.4公開schemaに許容されない識別子やfieldの追加が必要なら§9.1の1.5公開へ束ねる。安全性診断の修正と新layoutの公開時期を混同せず、書籍ゲートが未達の間にcapabilitiesで完了を宣言しない。

行頭の負originも新production bridgeで扱う。実engineの11pt数式は`origin_x=-0.6875pt`となり、spacing=0の式のみの段落を旧atomic-vector bridgeが空行overflowとして拒否することを確認した。候補行の`left=min(0, visual_left)`、`right=max(logical_advance, visual_right)`から必要幅を`right-left`として検査し、収まる候補のline originを`body_left-left`へ移す。これは行全体の配置補正であり、数式のorigin/advance/spacingを書き換えない。選択済みreceiptへこのoriginを含め、本文・SVG・タグ・リンクを同じ座標で投影する。実際の必要幅がbodyを超えた場合は依然としてoverflowとし、縮小やclipで隠さない。旧staging helperの凍結負例は新bridgeの正例で置換せず、別profile/recipeの試験として保持する。

実装追補: `typaxis-syntax/src/production_flow.rs`に`prepare_production_text_flow`を追加した。これは順序・構造境界・継承style・exact text・言語を保持するsyntax側の入力ownerであり、選択済み配置receiptではない。続いて`typaxis-shaping/src/production_text.rs`に`shape_production_authored_text`を追加し、admitted TT/TTC/CFFの実glyph・cluster・advance・hhea metricsを得る経路を検証した。参照labelは未確定ownerとして残し、解決後のcontext再shapeを要求する。共通行組み・改ページ・PDFへの接続は未完了で、現行production runnerの仮座標をこの入力の正しい配置と扱わない。検証結果は[実装台帳](28-vmb-book-production-progress.md)を参照する。

追加追補（2026-09-06）: `typaxis-layout/src/production_inline.rs`と`typaxis-linebreak/src/production_inline.rs`で、shaped clusterとbound SVGを同じLTR行候補に結ぶ経路を実装した。実VMB 11pt分数SVGの負originを保持した行頭補正、前後本文の実advance、cluster内部の改行禁止を検証した。soft/hard breakはsource owner/spanを持つ幅0項目として接続した。参照・native数式・bidiの未接続部分はowner付き保留エラーとして残す。共通改ページは下記の本文/SVG/caption cursorまで進んだが、全領域とPDF writerへの接続は未完了であり、この行候補の成功を全巻PDFの受け入れ証拠にはしない。

改行の契約は既存の`05-text-pipeline.md`と`07-paragraph-layout.md`に合わせる。soft breakは空白文字を生成せず、幅0の任意改行として扱う。hard breakは幅0の強制改行であり、Unicode分類の文脈もBKで切る。break nodeのowner/source spanをitemization fingerprintに含め、行範囲は改行項目を含むindexで記録する。段落末尾ではsoft breakも終端mandatoryとなり、trailing hard breakの後に空行を追加しない。連続hard breakによる空行には段落のcomputed line-heightを使う。SVG前後spacingは改行が選択されず同じ行に内容が続くときだけ加算する。shaping文脈用のhard break U+2028はglyph/source textとして出力しない。行境界で必要な再shapeとbidiの最終行並べ替えは別の必須段階として維持する。

改行だけの段落は、新linebreak kernelで行高と改行位置を保持できる。ただし既存tagged profileは`Paragraph { has_real_content: false }`を`UnsupportedSemantic`で拒否する。新production profileの接続時には、改行による空行のlayout ownerとpaintを持つsemantic fragmentを区別し、空行のためのdummy glyph/MCRを生成しない。空semantic containerの拒否を一律解除せず、改行だけの段落を含む実本文文書について独立したadmission・pagination・structure試験を追加する。

行内配置の追補（2026-09-06）: `typaxis-layout/src/production_selected_inline.rs`の`layout_production_inline_lines`は、prepared本文/SVG全段落と同数の幅を要求し、元の段落順で行を選択する。kernelが選択時に保持したunit penを本文にも使い、glyph originを`pen + offset_x`、`line baseline - offset_y`としてY-down座標へ投影する。SVGは同じline baselineとorigin補正を使う。source cluster・元run/glyph・font・改行ownerを保持し、内容を文字列から再構築しない。行組みの候補訪問数と配置record数は文書全体の一つの予算で制限する。これは行内の相対配置であり、page/frameの確定、行境界再shape、bidi、実ink bounds、justify、PDF描画を完了したreceiptではない。下記の共通cursorが選択したline topを本文/SVGの両方へ加える。semantic fragmentとPDFへの投影は引き続き実装する。

共通cursorの追補（2026-09-06）: `typaxis-pagination/src/production_body.rs`の`paginate_production_body`は、source flowの段落行、block SVG、vector caption、明示改ページを同じcursorで配置する。行とblockのpackage/profile/limits/admission/binding epochを照合し、段落の実行高とblockの実content heightを消費する。paragraphのstart/end indent、start/center/end alignment、before/after spaceを使用する。semantic containerは縦余白と末尾keepを子の最初/最後の配置へ伝える。左右indentは共通body frameを使う場合に親の利用可能幅から差し引き、子の本文・数式・図版・listへ渡す。frameを持たない旧行組み結果での非ゼロindentとnamed-page選択はowner付き保留とする（2026-09-07追補）。captionは実際の本文行を消費し、keep_caption=trueではblockからcaption末尾までの実高さを同じページに保つ。keep_with_nextは後続の実行高・余白を含めて判断し、groupが空ページにも収まらない場合はoversizeとする。明示改ページは先頭・連続・末尾を含めて1 nodeにつき必ず次ページを作る。keepと明示改ページの衝突は診断し、片方を黙って無視しない。

選択fragmentはparagraph/lineまたはblock index、page index、bounds、baseline、SVG viewportを持ち、本文glyphとinline SVGには同じline originを加える。page/fragment/work recordの有限予算を行・block preparationから継続し、既定の先頭blank pageもページ予算に数える。当初は実寸法に基づく前方配置だけだったが、§14.8の内部候補比較とwidow/orphan・heading costを接続した。汎用のpage-break trace/budget receipt、収束loop、table/footnote/native mathの共通配置とterminal/paint authorizationは未接続で、これらを全巻対応済みと扱わない。source flowが未接続領域に遭遇した場合は部分結果を返さず、所有nodeを診断する。最終publication前に既存のpage-break policy/trace/convergence契約へ統合する。

### 14.3 本文fontとPDF出力の設計

本文の全selected cluster usageを`StagingPdfTextClusterUsage`相当へまとめ、既存`finalize_staging_pdf_text_fonts`のTrueType/CFF subset経路へ接続する。現APIの公開constructorだけで任意glyphを信頼せず、production bridgeがshape receiptとadmitted fingerprintを照合したusageだけを渡す。本文・caption・式番号で同一faceを使う場合は一つのdocument font usage集合で課金・subsetし、呼出し単位で予算をリセットしない。native mathのglyph usageとの共有もfont instanceと元glyph/clusterのidentityで判断する。

全巻用のCID割当ては既存の式番号用recipeをそのまま流用しない。`typaxis-resources/src/staging_text.rs`のTrueType経路は、異なるsource clusterの各glyphへ新しいCIDを割り当てるため、同じ文字を反復する本文でも配置数に比例してCIDを消費する。production専用finalizerでは、同一font instanceの要求されたoriginal GIDを昇順に並べてCIDを一つずつ割り当て、各配置から共有CIDを参照する。TrueType compositeの依存glyphはsubsetへ収録するが、直接描画しない依存glyphに配置用CIDを追加しない。既存の凍結式番号recipe・出力hashは維持する。

CID共有と抽出テキストは別に管理する。まず単一glyph・単一Unicode scalarの用例から矛盾しないToUnicodeを作り、同じglyphが異なる文字へ対応する場合は一意に決めない。各clusterについてCID列のUnicode連結と元のexact textを比較し、一致しない場合はそのcluster全体へActualTextを一回付ける。glyph数と文字数が同じという理由だけで位置対応を推測しない。clusterのsource span・配置ordinal・選択済みglyph位置は共有せず保持し、本文の反復、ligature、結合文字で抽出内容が失われないようにする。

production font planは選択済みdisplay・admitted ledger・effective limitsに結びつける。別package/別配置のusageを混ぜる入力は拒否し、subset bytesとglyph/CID/cluster record予算を文書全体で課金する。face/GID/cluster照合にはindexまたは二分探索を用い、全配置について毎回cluster全件を線形探索しない。回帰試験には、異なるsource spanに同一文字を多数配置しCID上限を1にした正例、65,535回を超える反復配置、曖昧なUnicode対応のActualText、TT/TTC/CFF混在、owner/limits差し替え拒否を含める。これらは全巻の本文フォントを扱うための設計であり、現時点のPDF接続完了を示さない。

PDF writerはselected glyph位置とfrozen CID planから描画し、本文を再shapeしたり、Unicode scalarごとのcmap lookupで再配置したりしない。ToUnicodeは元clusterを保持し、必要なclusterにはActualTextを一回付ける。複数font・日本語・結合文字・ligature・IVSの処理を同じ原則で行う（IVS admissionの新規対応は§7の新profile）。空白advanceを一律0.6emに置換せず、選択済み行の空白・禁則・justify結果を使う。

数式SVGのForm共有、配置ごとのFormula/Alt/ActualTextは維持する。抽出補助が必要なextractorでは、見えないanchorを専用の管理されたglyph usageとして登録し、任意native数式の最初のglyphに依存させない。anchorはviewport/読み順/タグへ結び、可視ink・余分な抽出文字・二重ActualTextがないことを独立extractorで検査する。

実装追補（2026-09-06）: `typaxis-display-list/src/production_body.rs`で共通ページ配置を本文glyph/SVGのpage-space displayへ投影し、`typaxis-resources/src/production_body.rs`でadmitted fontとexact displayに結びつく共有CID計画を追加した。`typaxis-pdf/src/production_body_text.rs`はその計画から実font size・位置・CIDを描画命令へ変換する。実VMBの2ページ入力と65,536本文glyphのCID共有を検証した。これは本文描画contributionまでであり、式番号、最終object/structure/navigation plan、public writerへの接続と独立render/extractは未完了である。詳しい証拠と未接続範囲は[実装台帳](28-vmb-book-production-progress.md)の同日追補を参照する。

追加実装（2026-09-06）: `finalize_production_body_vectors`は同じ本文font/display ownerから内容キー順のForm計画を作り、`build_production_body_page_content`は本文とSVGを選択済みdraw順に同じページストリームへ結合する。SVGのmatrix/scale/currentColorはbound placementから取得し、FormにはAlt/ActualText/MCIDを入れない。共有Formのalias使用数と配置別の意味情報を別に保持する。空白ページにも一回だけpage root Y反転を置き、描画のないページへdummy glyph/Doを追加しない。最終structure ownerが各drawを正しいMCRへ結ぶため、source draw indexと描画byte範囲を公開する。これ自体をタグ付きPDFの最終認可や公開writerの完成とはしない。

5,000 alias試験のCPUサンプルから、言語検証等で各vectorの検査のたびに全packageの`checked_wire`を繰り返す二乗処理を確認した。`PrecomposedVectorVerification`は全体検査後のimmutable packageを借用する非構築可能なscopeとし、言語・metrics・styleの個別照合を同じscopeで行う。既存単発APIは従来の全体検査を保持する。scope内でも別owner/sessionのmetrics、改変language/styleは拒否し、scope作成時は他nodeにある破損も検出する。言語・構造・profile・layout・式番号shapingのbatchへ接続する。PDF側もpageごとの全usage再走査を一回のpage groupingへ変更する。これは§6.5の調査後に確認した追加の書籍規模性能修正であり、検証内容やaliasごとのresource admission課金を省略する変更ではない。

### 14.4 追加の必須回帰・完了条件

- native数式0の本文＋inline/block SVG、通常TrueType（MATHなし）でcheck/buildと独立render/extractが成功する。本文だけの入力も対応profileの範囲で同様に確認する。
- 負originを持つSVG数式だけの段落をspacing=0で行頭へ置き、行origin補正後のviewportがbody内に収まる。実engine 11ptで確認した負例を正例へ変える。前後本文があるケースだけでこの条件を代替しない。
- 異なるadvanceのLatin文字と和文を含む2種類のfont、複数font sizeで、本文とSVGのbaseline/前後spaceが共通line receiptに一致する。font宣言順の変更で未選択fontが採用されない。
- 「本文A→inline式→本文B→block式→caption→次段落」が狭いページを跨ぐ入力で、可視内容・抽出・structure読み順が一致する。本文だけの段落の欠落、page 0集中、式や本文の二重配置を拒否する。
- 見出しdestinationと複数行linkのpage/矩形が選択済み配置に一致する。タグがあるだけ、PDF bytesが生成された、画像数が一致しただけでは合格にしない。
- 旧native math-only authorizationの拒否境界、同一package以外のreceipt/selected-layout tamper、old-profile corpusを維持する。§8の5,000件・全巻・独立PDF検査を省略しない。


### 14.5 選択済み位置からのnavigation生成

インラインアンカーは、source owner/spanと、その位置までに準備したlogical unit数を持つ
非描画markerとして保持する。文字・数式・明示改行のunit列へは挿入せず、幅、UAX #14の
break候補、glyph cluster、抽出文字列を増やさない。同じgapの複数アンカーはsource順を保つ。
行境界と一致するgapは次の行へ所属させ、段落末尾だけは最終行のlogical advanceへ置く。
明示改行の直前はそのcontrolのpen、直後は次行のpenになる。行内では選択済みunit penと
origin shiftを使い、次の文字や式のglyph位置から逆算しない。

行位置はline index・x・baselineとして保持し、paginationが選択したparagraph-line fragmentの
x/yを加えてpage-spaceへ投影する。元のmarkerとpage/fragmentを結ぶprivate fieldのrecordを
描画列とは別に保持する。markerの追加でMCIDやDoを増やさず、paragraph/endやpage 0の仮座標へ
置換しない。準備・行選択・page projectionで各recordを文書全体の有限予算へ課金する。
行選択はmarkerとlineを一方向に走査し、page projectionはline順のmarker範囲を検索して
その行だけを読む。各行で段落中の全markerを再走査しない。

アンカーだけの段落は現在のtagged profileでUnsupportedSemanticとなることを確認した。
source flowはアンカーを保持するが、この追補の位置保持だけで受理範囲を変えない。
将来その段落を受理するときは非描画flow cursorから移動先を確定する処理とprofile検査を
同時に実装する。選択行がない状態をOptionの未確定位置として明示し、navigation builderは
解決できなければowner付きのunplaced-anchor診断を返す。空文字やdummy glyphを追加しない。

本文のlink用logical boundsは、選択済みcluster pen、実glyph advanceの合計、選択fontの
ascender/descenderとページbaselineから求める。offset付きglyphのink boundsとは区別する。
advanceが0なら正幅矩形を捏造せずNoneとし、後続のlink集約で他の実配置との関係を扱う。
数式のboundsは既存のviewportを用いる。source registryのLink子孫のこれらの矩形を、同じ
page/line fragment内で集約して注釈候補とする。行やページを跨ぐ巨大な矩形へ結合しない。
ゼロ幅内容だけのlinkなど有効な矩形が得られない場合は、明示的に未配置として診断する。

次に実装するnavigation ownerは、正確なselected display/structureとsource navigation registryを
借用して照合する。inline anchorは上記record、heading/block/container anchorはそのsourceの
最初の実selected descendant fragmentへ対応付ける。sourceのtarget名とoutline階層を維持し、
PDFのdestination座標へのY変換は実page heightから一回だけ行う。sourceにexternal linkが
存在する場合も黙って省略せず、そのlink種別を検証・出力する経路または明示診断へ接続する。

PDF接続時はdestinations/outline/annotationを型付きobject roleに追加し、絶対object番号を
割り当てる前に全graphの個数と参照を検査する。pageにはAnnots、catalogにはDests/Outlinesを
接続する。Link StructElemのKへOBJRを追加し、annotation StructParentはpage MCID用キーに
衝突しない範囲を使い、ParentTreeNextKeyも合わせて更新する。任意raw座標を受け取る旧staging
helperを新production配置の証拠として流用しない。これらのPDF接続と独立検査が完了するまで、
既存PendingNavigation拒否は外さない。terminal/paint/manifestと公開buildの接続はさらに別の
残件であり、非描画markerとlogical boundsの実装だけで全巻ゲートを完了したとは扱わない。


実装追補（2026-09-06、selected navigation接続）: `ProductionBodyNavigation`を追加し、上記の
private marker・選択済みfragment・source structure registryからdestination、行別link、outlineの
親子・前後関係を生成する。container/headingの最初のfragmentはregistryを逆順に一回伝播して
求め、各移動先で文書全体を走査しない。本文の論理矩形と数式viewportを同じpage/line内だけで
集約する。source targetと構造ownerが対応しない場合、実配置のないanchor/link、nested link、
算術・予算超過はowner付きの型付きエラーとなる。以前の一律PendingNavigation拒否は、この
ownerを検証してPDF graphへ接続する処理へ置き換えた。

`ProductionInlineSite`は、BeginLinkに限り検証済みの内部targetまたはURIを借用して保持する。
EndContainerへtargetを複製しない。`typaxis.production-text-flow/3`がその契約を区別し、元packageの
hashとflow再検証でtargetも結ぶ。URI actionは既存PDF writerと同じく検証済みURIの元bytesを
hex stringへ格納する。内部destination名とoutline titleはUTF-16BEとし、name treeは実際の
UTF-16 code unit順で並べる。URIだけの文書に空のDests treeを追加しない。

検査用PDFのcatalog/pageにNames/Dests、Outlines、Annotsを接続し、Link StructElemのOBJRと
annotationのStructParentを相互参照させる。page MCID用のParentTree配列は維持し、その後ろの
キーに注釈用StructElem参照を追加する。追加の構造index・link record・文字列・PDF objectを
既存の文書全体予算へ含め、全graphの参照を解決してからobject番号とxrefを確定する。
注釈にはBorder [0 0 0]を指定し、クリック領域のための可視枠やglyphを追加しない。

現物VMB数式のリンク、heading/containerの目次、3ページに分かれた内部リンクとURIリンクを
Rustと独立pypdf検査で確認した。独立検査はselected receiptに対するPDF座標・参照・タグ・
目次階層の一致を検証し、移動先・矩形・注釈・OBJR・ParentTree・目次cycle等の改ざんを拒否する。
この接続は選択済み共通flowが扱える範囲の検査用PDFであり、anchor-only段落のprofile/flow cursor、
汎用pagination、terminal/paint/manifest、公開check/build、正式VMB exporter、原ノ味・実全巻の
受け入れ条件を完了した意味ではない。最新の実行結果は実装台帳を参照する。


### 14.6 通常のPNG/JPEG図版とキャプションの共通配置

追加実装（2026-09-06）: 元全巻の通常`figure`は51配置、参照するPNGは48宣言だった。
`ProductionTextFlow`へ通常figureのsource owner/span、image ID、Alt、placement、解決済み
styleを保持する。prepared inline ownerからadmitted imageのhash・ピクセル寸法へ結び、
`width`の物理長とピクセル比で高さを一回だけround-half-to-evenで計算する。
この経路は`placement=block`、明示したwidth、PNGまたはbaseline JPEGを扱う。auto幅、
別placement、通常figureに指定したvector mediaは未対応理由を返す。named page、本文幅超過、
ページより高い図版も所有node付きで拒否し、暗黙の縮小・切断・用紙変更をしない。

共通paginationは通常図版を一つの実高さfragmentとして本文・数式と同じcursorに置く。
キャプションは実際に組んだ各行を後続fragmentとして保持し、`keep_caption=true`なら
図版から最後のcaption行までをkeepで結ぶ。falseなら通常の改ページ候補を使う。
figureのspace_beforeは図版の前、space_afterとkeep_with_nextは最後のcaption行
（captionなしなら図版）の後へ適用する。captionの行幅はその段落に解決したstyleとbody幅に
従い、画像のwidthへ黙って縮めない。list/table/footnote等を含む未接続caption subflowは
未対応診断を維持する。

PDF用のraster planは選択済みdrawだけから作る。同じstable bytes/hash/mediaの画像は、
異なるimage IDであっても一つのImage XObjectを共有する。論理宣言のadmission課金と
配置node、Alt、caption、MCIDは共有しない。PNGは色と必要なalphaを分離し、固定した
Rust deflate backendで可逆圧縮する。JPEGはadmission済みのnormalized streamと
ColorTransformの検証結果を使う。PDF page rootのY反転と画像scanlineの変換を区別し、
画像matrixを`[w 0 0 -h x y+h]`として元の上下方向を保存する。

records/spoolは既存font・vector・textの保持分から継続する。decode前に正規化pixel buffer、
色/alpha分離、明示したPNG decoder allocation ceiling、固定deflater workspaceと
圧縮結果の一時コピーを予算へ含める。writerは出力の上限を確保前に検査する。
`spool_charge`は戻り値が保持する圧縮payload、`peak_spool_charge`は処理中に検査した
一時workspace込みの高水位を示し、実測RSSとは区別する。同じ画像の再配置ではdecodeと
圧縮を繰り返さない。現時点の固定backendはflate2 1.1.9 / miniz_oxide 0.8.9で、既存PNG
dependencyのlock済み実装を明示利用する。公開profile/capabilityの変更は行わない。

通常図版は`Figure`のMCIDとsource Altへ結び、captionはその構造子として後続する。
Formula用のActualText補助glyphを通常図版へ付けず、図版のAltを抽出本文へ混入しない。
最終オブジェクトにはImage/必要なSMaskを割り当て、各ページのResourcesは実使用した
画像のみを参照する。この接続は検査用PDF経路であり、公開writer、terminal/paint/manifest、
正式VMB exporterと実全巻ゲートは引き続き未完了である。検証記録は実装台帳を参照する。

### 14.7 リスト内の本文・数式と生成ラベルの接続設計

設計追補（2026-09-06）。保全した元全巻には16個のlist、58個のitemがあり、最初の
list ownerは2908である。item内の本文と数式も全巻の必須対象に含める。本節は完成時の
接続仕様であり、検査用の共通配置・PDF経路への接続状況は末尾追補と実装台帳に記録する。
公開writerと全巻ゲートの達成とは区別する。

**sourceと文字生成。** `ProductionTextFlow`にlistのowner、ordered/start、解決済みstyle、
page_name、itemのowner、親list、item ordinal、source span、languageを保持する。
markerは既存のlist規則に従い、orderedはcheckedな`start + ordinal`とピリオド、
unorderedはU+2022とする。末尾空白をmarker本文へ追加しない。markerはitemをownerとする
`GeneratedBufferKey(ListMarker, ordinal=0)`に結び、authored paragraphや元source bytesへ
挿入しない。生成文字列の確保前に桁数・UTF-8 bytesを計算し、既存parsed textと合算した
text予算、per-buffer予算、record予算を検査する。番号overflowはitem owner付きで拒否する。
段階的なgenerated overlayは完全なgenerated storeの代用とせず、最終convergenceで
他の生成文字列と統合し、source由来のkey/textと最終reference fingerprintを再検証する。

**実フォントと幅。** `typaxis-shaping`は解決済みlist font family/size、実admitted face、
effective languageを使ってmarkerをshapeする。本文と同じglyph coverage・cluster・
missing glyph検査を行い、実glyph advanceの合計を幅とする。U+2022がないfontを空白や
別記号で代替しない。生成spanを保持したrun、選択face/hash、metrics、glyph/cluster列を
fingerprintへ含め、本文と共通の有限record予算から課金する。

`typaxis-layout`はlistごとに全itemのmarker実幅の最大値を一度計算し、右揃えのmarker列を
作る。markerとitem本文の間隔は既存規則のlist font size 1 emであり、PDF用の空白文字では
ない。親の利用可能幅からlistのstart/end indent、marker列幅、間隔を引いてitem frameを
決める。nested listは親item frameを基準とする。9から10へ桁が増えても同じlistの本文左端は
一致する。各段落はitem frameから自身のindentを引いた幅で改行を選び、配置時も同じframeを
使う。markerだけで幅を使い切る場合はlist owner付き`ListFrameExhausted`相当で拒否する。

**ページ選択。** `typaxis-pagination`はlist/itemをsource順の共通flowに接続する。
各itemのmarkerを、最初の実描画fragmentとその選択pageへ一度だけ結び付ける。
本文行で始まるitemではその行のbaseline、block数式ではその実baselineに揃える。
baselineを持たない図版で始まる場合は、最初の描画領域上端とmarker ascenderからbaselineを
決める。markerのascender/descentが最初のfragmentより大きければ必要な上下量をページ消費高へ
含め、本文・SVGの元の高さやline-local座標は変更しない。nested labelが同じfragmentへ付く
場合は必要量の最大を使い、各labelの横位置とsource順は独立に保持する。

itemが次ページへ継続してもmarkerを再描画しない。先頭に明示page breakがある場合はそれを
処理した後の最初の実fragmentへ結ぶ。描画fragmentを持たないitemはowner付きで拒否し、
marker用のdummy段落を作らない。listのbefore/after/keepは最初・最後の実fragmentへ適用し、
markerを含む先頭行がページより高い場合もoversizeを返す。汎用lookback/widow/収束規則への
接続は§14.2と同じ公開前ゲートであり、前方配置だけで代用しない。

item内の通常図版はitem幅でwidthを検査する。block数式・式番号・captionも同じitem frameを
渡して再準備し、body全幅で計算した結果を横移動するだけの接続にしない。captionの段落幅は
解決済みitem frameと自身のstyleに従う。未接続table/footnote等は引き続きそのownerで診断する。

**displayとPDF構造。** `typaxis-display-list`は選択したmarkerのfont・cluster・位置を
通常の本文と共通のfont/CID/subset経路へ渡す。ただし生成source keyはauthored spanと区別し、
display buffer IDを割り当てる場合も両namespaceの衝突と整数overflowを検査する。
既存の`L → LI → (Lbl, LBody)`のLblへmarkerを結び、item本文と数式はLBodyへ残す。
LblのActualTextはcanonical markerそのものとし、1 itemにつき1回のpaintを要求する。
続きページやSVG共有によってLbl、Formula、MCIDを複製しない。marker位置・生成key・font・
選択fragmentをreceiptと最終manifestへ含め、別itemへの差し替えを拒否する。

追加の必須試験はordered 9→10、unordered、nested list、複数行item、ページ継続、先頭の
inline/block分数、図版とcaption、本文より大きいmarker font、番号overflow、glyph不足、
幅・高さ・text/record予算の境界とする。視覚試験ではmarkerと本文glyphに非空輪郭のある
固定fontを使う。独立PDF検査でmarkerの回数・位置、抽出順、Lbl/LBody/FormulaとParentTreeを
確認し、marker欠落・重複・別item参照・誤baselineを改ざん負例にする。元全巻の16 list/
58 itemと各item内の本文・数式のinventory一致も全巻ゲートへ加える。

VMB側の責務は[VMB設計§14.7](../../../v/vmb-container/docs/typaxis-book-export-design.md)に
記載する。VMBはlist構造と元の内容を渡し、番号の可視文字・PDF座標・Lblを先に生成しない。


実装追補（2026-09-06）: sourceが生成したmarkerをadmitted fontでshapeし、
`layout_production_body_inline_lines`で実marker列幅とnested item frameを計算する経路を追加した。
共通paginationは最初の描画fragmentへmarkerを一度だけ配置し、先頭の空のhard-break行や
明示page breakはラベルの描画先にしない。大きなラベルの上下量を消費高へ含め、同じ先頭行に
付くnested labelは必要量の最大を一度だけ課金する。block数式の利用可能幅とstart/center/end
整列は実item frameで再計算する。式番号付きblockの再準備は未接続であり、拒否を維持する。

選択済みmarkerのgenerated key、実glyph/clusterと位置をdisplay→共有font/CID→PDFへ接続し、
既存のLblへ結ぶ。Lblの置換文字列はそのMCID内のSpanに一度だけActualTextとして出し、
StructElemへ重複して付けない。LBodyには元の本文・数式・図版・captionが残る。
source flow /5、authored shape /3、body pagination /3、body display /4、body structure /3へ
内部identityを更新した。公開contract/profile/capabilityの変更は含めない。

基礎の11試験、4種の独立PDF検査、36件の改ざん拒否を実装台帳へ記録した。これらは
検査用の選択済み経路の証拠であり、元全巻58 itemの公開build、式番号、generated storeの
最終収束、汎用pagination、terminal/paint/manifest、正式VMB exporterと原ノ味の各ゲートは
引き続き未完了である。


## 15. フォント診断の実装追補（2026-09-06）

§5.3・§7.6の詳細CFF admissionとcontainer/face診断をresource admissionおよび
公開check/buildへ接続した。位置は`offset_kind=field`（特定したfield/operator）と
`offset_kind=context-start`（解析単位の先頭）を区別し、未知のoperand位置を推測しない。
現物原ノ味の両コマンドは`VORG`・`font_byte=108`・`fsType=0`を表示する。
失敗manifestで部分受理済みresourceのmedia宣言を落とす不具合も修正した。
検証コマンド・証拠・未対応範囲は[実装台帳](28-vmb-book-production-progress.md)の
「CFF admission and TTC diagnostics through public check/build」を参照する。
この追補は原ノ味の受理範囲や§10の全巻合格条件を変更しない。


### 14.8 実測された本文fragmentからの改ページ候補選択

`production_breaks.rs`を共通body cursorの内部stageとして設ける。入力は既に行組みした
paragraph line、block SVG、図版、captionとlist markerの必要高さを反映したItem列であり、
sourceの文字数や画像数から行高を推定しない。`typaxis.production-body-pagination/4`に
内部選択方針`typaxis.production-body-break/1`を結び、そのidentityと全候補をfingerprintへ含める。
公開profile/contractの切替や、汎用収束stateの発行を意味する識別子ではない。

選択手順は次のとおり。

1. keep chainの実高さを逆方向に一度計算し、空ページを超えるchainと明示改ページを跨ぐkeepを
   先に拒否する。SVGやcaptionの分割方法をこのstageで変更しない。
2. 現ページの最初のItemから、EOF、明示改ページ、または実高さのoverflowまで走査する。
   ページ先頭のbefore spaceは捨て、内部境界だけ前Itemのafterと次Itemのbeforeを加算する。
   list marker用leading/trailingは一度だけ加える。
3. EOF/明示改ページまで全て収まる場合は、その必須境界一つを採用する。それ以外は、
   収まった各Itemの後ろの境界を逆順に評価する。keepの付いた境界は候補にしない。
4. 非keep候補の評価・確保前に`max_page_break_lookback`を消費する。全候補を調べられない場合は
   `PageBreakLookbackLimit { limit, observed }`と境界ownerを返し、途中までの最良候補を返さない。
5. 候補はsource順に保存し、以下のcost合計が最小の境界を採用する。同額ならend Item indexが
   小さい方とする。次のページを同じ入力列の続きから開始する。

| component | 内部方針 /1 の計算 |
| --- | --- |
| widow_orphan | 段落内部で切る場合、現ページ側が2行未満なら1,000,000、残りが2行未満ならさらに1,000,000。ページを跨いだ段落は現在のページに載る行だけを数える。 |
| heading_isolation | 候補直前がheadingの行なら2,000,000。heading途中の分割とheading直後の孤立の両方を避ける。 |
| unused_space | `floor((body_height - used_height) * 1000000 / body_height)`。固定小数点のraw値とcheckedなi128中間演算を使う。 |
| keep / table_split / footnote_split / overflow | 0。keepは候補禁止、未接続subflowは上流の保留診断、overflowは候補不成立として扱う。 |

必須境界のcostはすべて0。widow/orphanとheadingの条件はsoftな選好であり、2行入らない本文高で
文書を消失させない。将来の全stateの最適化やtable/footnoteのcostを、この局所方針で代用しない。

各decisionはpage index、開始Item、全候補の終端Item/owner/used height/component cost、
選択候補index、終了理由を保持する。Item・補助paragraph/heading情報・decision・candidate・
配置fragmentは同じ有限record予算へ課金する。選択後のmaterializationはdecisionの範囲だけを
配置し、ページ末尾で使用高さが選択候補と一致することを確認する。明示改ページの先頭・連続・
末尾による空ページも従来どおりページ予算へ数える。

VMBの書籍用configには`max_page_break_lookback = 128`を追加する。既定32のままでは、
33個の候補がある普通の本文ページも拒否されるためである。128は書籍用の明示policy値であり、
一般defaultの変更や全入力の成功保証ではない。70行/33行収容の合成入力で32の拒否と128での
33+33+4行配置を検証する。実全巻の候補最大数・総数・最大RSSは別途測定し、必要なpolicy変更は
package/configの新しい試験条件として記録する。失敗後の暗黙増枠・自動再試行はしない。

内部decisionは汎用`PageBreakSearchBudget`や`LayoutPassCoordinator`の真正なreceiptではない。
公開runnerへ接続する際は同じ候補順・選択結果を汎用のflow boundary/epoch/traceへ結び、
上限超過をG6xxx診断へ伝え、生成文字列の再計算と最低2passの収束検査を行う。この接続、
最終line reshaping、terminal/paint/manifestと実全巻の公開check/buildは依然として必須残件である。


### 14.9 入れ子のsemantic containerの実幅

`layout_production_body_inline_lines`のframe走査は、semantic containerに入る前の
親frameを保存し、解決済みstart/end indentを差し引いた正の幅を子へ渡す。
コンテナの終端で親frameへ戻し、後続の兄弟を内側の幅で組まない。
本文の改行と共通paginationは同じframeを用い、段落固有のindentはその後に一度だけ
適用する。block SVGはその実幅で整列を再計算し、PNG/JPEG図版のwidthとcaptionの
段落幅、list marker列とitem frameも同じ親frameを基準とする。

幅を使い切るcontainerは`ContainerFrameExhausted`とそのownerで拒否する。
任意幅で先に組んだ旧行結果を後から横へ移す接続は受理せず、非ゼロindentには
測定済みbody frameを要求する。親・子のframe、復帰順序と元のprepared identityを
`typaxis.production-body-frames/1`の内部fingerprintに結ぶ。公開profileは変更しない。

この実装は共通配置経路の幅伝播を閉じる。式番号付きblockのitem/container幅での
配置は§14.11で追補する。named page、最終行reshapingと汎用収束、公開terminal/paint/manifest、
公開全巻buildは別の必須残件であり、本節の試験でそれらの完了を代替しない。

### 14.10 共通body fragmentに結び付けるblock数式の完了記録

`finalize_production_body_math_terminals`は共通paginationの選択結果を消費し、
そのblock準備で使用した真正な`StagingMathVectorFlowRegistry`のterminal ledgerを閉じる。
別registry、別layout epoch、別予算、二度目のfinalizeは拒否する。各block数式の実fragmentの
owner、flow identity、ページ、viewport、baselineとcontent heightを確認した後にterminal 1を
一度消費し、registryに残る未消費flowがあれば成功しない。改ページ、caption、vector figureや
inline数式をblock terminalとして数えない。式番号の準備だけでは完全な選択を意味しないため、
§14.11の真正な番号配置を同じfragmentから選択した後にterminalを消費する。
番号付きblockの描画にその選択結果が渡されない場合は`PendingEquationNumber`を返す。

ledgerの補助recordとreceiptは既存の累積fragment予算へ課金する。文字列の検証・生成に
必要な保守的な上限をspool予算で事前検査し、生成後に保持するcanonical JCSの実byte数を
font、content、object、PDF組立てへ引き継ぐ。元のplacement fingerprintとterminal setの
fingerprintと番号の選択矩形を`typaxis.production-body-terminal/2`で結ぶ。
/2は番号のない初期stage /1を置き換える内部識別子であり、公開contract/profileは変更しない。

これはblock数式の配置完了stageであり、汎用収束のreceipt、paint認可、公開writerの
`VerifiedPdfBytesReceipt`を発行しない。公開pipelineへの接続と§10の全巻検証は必須残件のままとする。

### 14.11 共通fragment内の明示式番号

共通bodyのblock数式について、同じ準備registryの封印済み番号shapeを借用し、
親fragment index・page index・親owner・番号owner・shape fingerprint・番号矩形を保持する。
番号の右端は実際のinner frameの右端、topはcontent height内の既定の垂直中央位置とする。
数式viewportの整列は既定のtext-alignを維持し、番号を入れるために勝手に左へ動かさない。
container/listで幅が縮んだ場合は式と番号のminimum gapをその幅で再検査し、衝突を拒否する。
式と番号の両方を選択した後にatomic terminalを消費し、強制改ページでも二重に消費しない。

番号shapeのglyphを共通displayへ投影し、本文と同じfont usage／共有CID／content／objectの
経路で描画する。数式のActualTextへ番号を連結せず、既存の式番号bindingを持つ独立Spanへ
MCIDを割り当てる。番号Span自身に重複するstructure-level ActualTextを追加しない。
各clusterの選択位置・text span・exact textと真正なshapeを結び、PDF marked-content側が
番号全体の文字を一度まとめる。visual run位置にはbidi levelの反転を適用し、描画の所有順は
source cluster順を保つ。これは段落の最終line reshapingや汎用bidi行選択の完了を意味しない。

番号baselineはadmitted fontの実hhea ascent/descentをfont sizeへ変換し、
`top + (line_height - ascender + descender) / 2 + ascender`とする。raw固定小数点で
checked演算し、整数除算はゼロ方向へ丸める。line heightがfont extentより小さい場合の
負のhalf-leadingも保持する。glyphの上向きoffsetは下向きページ座標で引く。
従来の凍結staging writerのbaseline recipeは変更しない。

番号の配置record、visual-order/run位置の補助record、glyphコピーとdrawは共通の累積予算へ
加算する。内部displayは`typaxis.production-body-display/5`、structureは
`typaxis.production-body-structure/4`で識別する。これらは診断用PDFの組立てまで接続済みだが、
public writerの認可とmanifestへの接続、正式VMB全巻出力と独立全巻検査は必須残件である。

### 14.12 実エンジン5,000式の公開admissionと行頭overhangの再現

VMB生成器の`-corpus distinct-5000`は§8.5の数式templateを実engineへ渡す。
Typaxis側の`tools/prepare_vmb_distinct_probe.py`は5,000件のTeX/speech、raw/derived hash、
path形状のdistinct数、engine artifact、11ptと既存decimal6（最近接・tie-to-even）寸法を照合する。
このhelperは試験packageのauthoring harnessであり、正式RenderBook exporterではない。

同helperはdistinct 5,000、4,952 SVG＋48 PNG、5,000 alias宣言＋8,000配置を複数containerの
一packageとして作る。全宣言を実配置する。3ケースとも公開checkが既定の画像/vector予算で
成功したが、distinctの公開buildは最初のinline式でL5100となり、PDFを発行しなかった。
当該式はadvance 1,918,708、viewport幅2,008,820、origin_x -45,056（すべてraw固定小数点）。
本文幅28,689,280に対する横幅超過ではなく、旧atomic line測定の`visual_left >= 0`条件が
負のoriginを拒否する。1式だけに絞った同じ入力でも再現する。

式のoutline、metrics、spacingを変更したり、inlineをblockに変更したりして受け入れ結果を
作らない。共通の行配置で実際のparagraph/frameと視覚範囲を扱い、公開出力へ接続する必要がある。
既存の凍結行選択器にはoverhang拒否のテストがあるため、その期待値を単に緩めて済ませない。
`tools/verify_vmb_scale_pdf.py`は各宣言の実使用、source順のFormula/Figure、Do、共有Form、
BBox、MCID/ParentTreeと二つのextractorを検査するために追加したが、このscale PDFはまだ
生成できておらず、同verifierのscale PDF成功は未検証である。詳細は実装台帳を参照する。

### 14.13 選択済み行の実コンテキストによる再シェーピング

`production_selected_line_contexts`は実際に選択したcluster、vectorと明示breakから
段落ごとのUTF-8行末offsetを導出する。本文文字をそのまま数え、inline objectはU+FFFC、
hard breakはU+2028、soft breakは空文字として、初期shaperと同じ段落コンテキストを用いる。
owner、段落数、最終offset、単調性とgrapheme境界を再検証し、欠落・別owner・cluster途中の
切断を拒否する。空行に必要な重複offsetは保持する。

`reshape_production_authored_text`は選択行の境界で実shaping runを分割し、backendへ渡す
pre/post contextもその行内へ制限する。実font、source span、script、段落のitemizationを
保ち、字形やadvanceを後から書き換えない。選択境界は
`typaxis.production-line-context/1`でshapeの内部identityへ結び付ける。
初期paragraph shapingの既存identityは変更しない。

`with_converged_production_body_lines`は初期shape/本文frameでの改行と、真正な
`LineReshapeFeedback`のpermitを消費する再shape/再改行を所有する。
比較には選択結果のfingerprint（shape、字形位置、改行、frameを含む）を用い、行数だけでは
安定と判定しない。初期shapeは行境界identityを持たないため、成功には少なくとも2回の
最終行reshapeが必要となる。候補探索のwork budgetは初回と全reshape passで共有する。
同一状態を実際に再生成した場合だけ`ProductionConvergedBodyLines`をcallbackへ渡し、
未収束・quota超過・shaping/layout失敗時にはconsumerを呼ばない。借用した選択結果を
callback外へ持ち出すために再計算したり、自己参照ownerを偽造したりしない。

原作fixtureのOpenType caltで、改行前後にA→Cの字形変更とadvance 600→900が発生する。
狭い幅では制限contextで安定し、中間の幅では1行と2行を往復する。この実測振動で
pass上限の停止を検証する。fixtureはHaranoAjiや全巻の代替ではない。

本stageは共通LTR行選択のfeedbackを閉じる。最終bidi行規則、全stageをまたぐ累積allocation
課金、page/generated-reference convergence、公開terminal/paint/manifestへの接続は別の
必須残件である。各stageの既存fragment ceilingと有限pass数は維持しているが、それだけで
全pipelineのallocation予算を閉じたとは扱わない。公開profile/contractは変更しない。
§14.12の負originについては、共通行選択器は既にvisual範囲に応じてglyphとvectorの両方へ
同じorigin shiftを適用する。公開旧行選択器の拒否を緩めず、この共通経路へ接続する。


### 14.14 共通本文graphのlifetime ownerと実exporter package

`with_production_common_body_pdf` は同じpackage/navigation/profile/host-admitted ledger
から本文flow、vector bindings、math registry、block準備を所有する。
`with_converged_production_body_lines` の実安定callback内で共通paginationを行い、
同じmath registryのblock terminalを閉じ、その選択から本文display、font使用、
page content、structure、marked content、objects、検査用PDFまで借用関係を保つ。
consumerへ渡すのは最後まで検証できたassemblyと、実pass数・候補数・選択/flowの
fingerprint・block terminal数の観測値である。未収束/予算失敗でconsumerを呼ばない。
このownerはnative math fontを本文fontの代用にせず、実選択された本文fontを使う。

新ownerは公開writer/manifestのreceiptではない。公開CLIへの接続、page/generated
referenceの汎用収束、最終bidi規則、全stageの累積allocation課金、table/footnote/native
mathを含む全flowの閉包は引き続き必要である。検査用assembly bytesをそのまま
VerifiedPdfBytesReceiptへ変換したり、既存manifestのownerに付け替えたりしない。

VMBのStageBookPackageから生成した、本文・見出し・ordered list・inline/block数式の
小規模jobを新ownerへ渡した。admission用fixture fontは見出しprefix「1」のglyphがなく
拒否されたため、その入力を保全し、Arial Unicodeを明示した新しい生成jobで再検査した。
新jobでは2回の最終行reshape、84候補steps、1 block terminal、1 pageの検査用PDFが
成功した。Poppler/MuPDFの空白正規化抽出は元packageのsource順の本文・番号・
ActualTextと一致し、両rendererで日本語と2式の可視描画を確認した。
この入力にはequation numberがない。番号付きの共通owner接続は別のunit fixtureで検査する。

同じ新jobの公開checkは成功したが、公開buildはexit 4 / I9190 native math mismatchの
ままで、PDFを発行しなかった。全input hashはbuild前後で一致。公開writerにはnative
math fontの先頭を標準本文描画へ使う経路が残るが、I9190の特定return siteまで断定しない。
実fontへの暗黙置換やdummy native mathでこの公開経路を成功扱いにはしない。
詳細hash、保存job、検査PDFとlogは実装台帳とVMB設計§15.38を参照。


### 14.15 静的本文のページ配置一致と累積pass予算

`paginate_stable_production_body` は同じ選択済み行・block準備を使うページ探索を
最低2回実行し、ページ数だけでなく配置・改ページdecision・list marker等を含む
既存の選択fingerprintを比較する。`max_layout_passes < 2` では成功を返さない。
各passのrecord費用と観測recordを、同じ`max_fragments`から累積して差し引く。
現在の保守的課金は各passの通常input chargeも再加算し、保持していない前passの
作業を予算から消さない。従来の単一pass APIの配置と予算は変更しない。

一致した結果は`ProductionStableBodyPages`と、実際の行/block準備への借用を持つ
`ProductionBodyPageStability`として渡す。別の準備、同じ配置でも前pass費用を
含まない選択は拒否する。block math terminalの付与後は、その元placement
fingerprintを照合するため、完了処理でfingerprintが変わっても確認できる。
共通PDF ownerはterminal後とPDF組み立て後に照合し、同じproofを後段callbackへ渡す。

これは動的参照のない本文についての静的ページ選択の一致であり、汎用の文書全体
収束receiptではない。reference/footnote referenceは元ownerで明示拒否する。
page/generated-reference・named page・table/footnote/native mathを含む全flow収束、
全pipeline allocation、公開terminal/paint/writer/manifestの接続は残件である。
2回同じ入力を組んだ結果を、まだ生成していない参照文字列の収束証明にはしない。

40レコードの単一pass fixtureでは、新ownerの2passと観測が82レコードとなり、
82で成功、81および41で失敗する。pass上限1では拒否、2で成功する。別準備の
拒否とterminal後の照合も検査する。実VMB小規模jobでは2 line reshapes /
2 page passes / 266 page records / 84 line candidate steps / 1 block terminal。
検査用PDFは§14.14のPDFとバイト単位で一致し、描画や抽出を変更していない。


### 14.16 共通本文の参照元情報

`ProductionInlineSite::reference()` は、通常参照の元target・要求format
（text/page/number）と、同じ検証済みnavigationのanchor所有ノードを保持する。
脚注参照は元footnote IDを保持する。文字列はpackageから借用し、参照先や番号を
本文文字列へ置き換えない。通常文字・link・container終端には参照情報を付けない。
anchorの検索はnavigationの整列済みregistryで行い、不一致時は参照元ownerで拒否する。

flow検証は元package/navigationの同一性と再構築した全paragraphの一致を確認するため、
target・target owner・format・参照種別・脚注IDの改変を拒否する。既存fingerprintは
全package SHAを含み、新情報も同じpackage/navigationから決定されるため、この追加で
algorithm識別子や公開contract/profileを変更しない。

これは生成前の参照情報の接続である。ページ番号・counter・脚注番号の解決、生成文字の
shapeと行選択、ページ再配置を含む収束、公開writerの接続は別途必要である。
既存の未解決参照の拒否を解除せず、参照を空文字としてPDF成功に扱わない。


### 14.17 脚注番号の生成namespaceと共通段落shape

共通本文flowは検証済みの脚注定義順から1-based decimal番号を作り、各参照ノードと
定義ノードそれぞれの`FootnoteMarker` keyへ保持する。同じ脚注を参照しても生成bufferの
ownerは別であり、本文text bufferのIDへ置き換えない。参照の出現順から採番しない。
生成文字とprovenanceはflowの再構築検証に含め、番号の改変を拒否する。

本文shaperは脚注参照の実生成文字を段落コンテキストへ入れ、他の本文と同じitemization・
選択font・language・行コンテキストでshapeする。番号だけの段落もfontとglyphを持つ。
再shapeで分割されたrun/clusterは生成namespaceの正しい部分範囲を保持する。
通常参照は引き続き未解決であり、脚注も定義領域・参照と定義の配置が未完了なので
pendingとして残す。ここで得たglyphは、脚注付きPDFの完成や公開paint許可ではない。

生成番号の文字列確保前に、list labelと同じ文書の生成テキスト予算へ課金する。
この境界の検査で、従来の共通flowが本文text bufferだけを保持量としていたことが判明した。
parserで計上した本文・native math speech・vector代替テキスト等の実保持量を引き継ぎ、
metadata・outline label・navigationの言語chargeも加える。既にparserで計上した
vector languageは重複加算しない。生成分と合わせた上限のちょうど内側／1 byte不足を
検査し、後者は最後の脚注定義ownerで`TextLimit`を返す。

生成内容とshapeのsource encodingが変わるため、内部algorithmは
`typaxis.production-text-flow/6`と`typaxis.production-authored-text-shape/4`へ更新する。
公開contract/profileと旧writerは変更しない。ページ参照の生成・収束、脚注領域への
共通配置、全pipelineの累積allocation、公開PDFの接続と全巻受け入れは残る。


### 14.18 生成された脚注参照の行配置

共通inline準備と選択済みclusterのsourceは、parsed `TextSpan`とgenerated provenanceを
区別する`ShapeSourceSpan`を保持する。脚注参照はflow所有の番号文字列と生成範囲を使い、
実shapeのclusterを本文と同じlogical unit、幅、baseline、改行探索へ接続する。
部分範囲の検査ではparsed buffer ID、またはgenerated keyとbuffer IDの一致を要求し、
範囲外・別owner・parsed/generatedの取り違えを拒否する。既存の非0開始parsed spanも
相対byte範囲へ変換してから照合する。

選択した行から作る次のshape contextには生成番号の実UTF-8長を含める。
脚注参照を含む段落について実際の選択→再shape→再改行の安定callbackまで検査する。
本文displayも型を保持し、生成bufferのdisplay IDはparsed buffer数にgenerated IDを
加え、`generated_provenance`を残す。parsed buffer数の検証は必要時に一度だけ行う。
これは脚注の最終構造・PDF許可を発行する変更ではない。

`A 1B`の1行／2行選択、同じbaseline、実glyph、生成範囲、選択contextの長さ、
別namespaceと別ownerの拒否を検査する。定義を本文へ連結することは許可せず、
単一pass paginationは脚注定義ownerで`PendingRegion("footnote")`、静的page proofは
参照ownerで`PendingRegion("generated_page_feedback")`を返すことも検査する。

内部algorithmはinline準備`/5`、inline行配置`/4`、本文display`/6`となる。
次の必須接続は脚注定義領域・参照ページと定義の対応・継続脚注の選択、その後の構造と
公開PDFである。ページ参照/counterの汎用収束、全巻・原ノ味・両hostの受け入れも残る。


### 14.19 脚注定義と選択済み参照位置の対応

`prepare_production_footnote_lines` は、同じ共通flowの検証済みwireに残る脚注IDと、
選択済み行の実generated clusterを結び付ける。`ProductionFootnoteLines`は実際の
選択済み行を借用し、別の選択結果や別limitsへの付け替えを拒否する。
定義ごとにID・owner・定義順番号・Begin/Endを含むevent範囲・paragraph範囲を保持する。
参照ごとには定義index、参照元が脚注定義内かどうか、実配置の最初／最後のcluster位置を
保持する。同じ脚注を複数回参照しても、参照ownerと配置は個別に残す。

参照のgenerated key/buffer/rangeとflow所有の番号を照合し、先頭byteから末尾byteまで
重複・欠落なく選択済みclusterで覆うことを要求する。複数clusterに分かれる「10」も
最後のclusterまで記録する。定義の範囲は共通event列から取得し、本文へのspliceにしない。

選択済み行のrecord chargeに、定義recordと参照record・検査用coverage recordを加え、
確保前に同じmax_fragmentsを検査する。1定義・1参照では追加3 recordsとなり、
必要数ちょうどで成功、1不足で拒否する。安定した行を渡すcallbackがこのregistryも所有し、
共通PDF ownerはページ処理の前に同一性を照合する。

これは参照ページへの割当・脚注領域の幅/高さへの適合・継続脚注の選択を完了するものでは
ない。次のページownerはこの対応とrecord chargeを引き継ぎ、定義領域の選択と本文ページの
決定を閉じる必要がある。現在の脚注pagination拒否と公開PDFの未完了状態は維持する。


### 14.20 宣言された脚注領域での共通行選択

脚注定義を含む共通frame準備は、同じpackageの単一default masterから実脚注領域を取得する。
呼出側の本文矩形が宣言と一致すること、領域が正寸法かつページ内であることを検査する。
脚注領域の欠落と、未選択の複数master／selection ruleは明示的に拒否する。
脚注を持たない入力にはこの追加のmaster制約を課さない。

脚注定義のBeginで本文frameを保存し、脚注領域の幅・本文原点からの相対横位置へ切り替える。
定義内の段落・入れ子container・listにはそのframeから既存のindent処理を適用し、Endで
前のframeを復元する。本文と異なる横位置・幅を受理し、本文より左の領域も符号付きの
相対位置として保持する。選択→再shapeの収束でも同じ宣言幅を使用する。

`ProductionBodyInlineFrames::footnote_region()`は宣言された最大領域を返す。
実際の使用高さ、本文との衝突解決、参照ページへの割当、継続脚注、定義番号のpaintを
証明する値ではない。領域recordを確保前の予算に加え、内部frame識別子を
`typaxis.production-body-frames/2`へ更新して横位置・縦位置・幅・高さをhashへ含める。
脚注のページ配置と公開PDFは引き続き未完了である。


### 14.21 本文と脚注定義の配置項目の分離

`prepare_production_body_flow` は、実際の選択済み行・block準備・既存の脚注参照registryを
借用し、同じpackage・binding・epoch・profile・admission・limitsに属することを照合する。
registryを再構築せず、その保持量から配置項目と定義範囲・list marker bindingを追加課金する。
脚注定義がある場合は、宣言脚注領域に結び付いたframeを要求する。

従来の本文paginationと共通の収集処理を使い、行・数式block・raster・list・containerの
実寸法、余白、keep規則、markerの上下追加量、明示改ページを保持する。registryの
定義ownerとBegin/End event境界を照合し、本文と各定義を別のsliceとして公開する。
定義を本文末尾へspliceしない。全体のsource indexは保持するため、行・block・markerは
同じ選択結果へ戻れる。値の任意構築・書換えや他の選択への付け替えを許可しない。

これはページ割当や脚注高さの適合判定ではない。高さ上限を超える複数項目でも収集できる。
次のページ探索は、本文と各定義の境界を保って、参照ページ・予約高さ・本文との衝突・
継続脚注・keepと改ページの規則を決定する必要がある。単一passの既存本文paginationは
引き続き脚注定義を拒否し、静的page proofも生成ページfeedback未完了を拒否する。
公開profile、PDF paint許可とmanifestの状態は変更しない。


### 14.22 脚注の内容断片と継続境界の探索

`prepare_production_footnote_search` は同じ実測flowを借用する探索ownerを作る。
`begin`で取得する`ProductionFootnoteCursor`は定義indexと次の項目をflowに結び付け、
任意の整数や別flowの位置を継続位置として受け入れない。断片選択から返る継続位置で、
内容の重複・欠落なく次の断片へ進める。同じ位置を別の利用可能高さで再評価できるが、
このowner内の候補保持量と訪問項目数は巻き戻さない。

本文の境界選択からページ番号に依存しない部分を共用し、実際の消費高さ・項目間の
前後余白・keep・段落の孤立行cost・heading costで切断候補を評価する。断片の先頭では
space-beforeを抑制し、末尾のspace-afterを消費高さへ足さない。複数定義を同じ領域へ
置く際の定義間spacingは、ページownerが別途予約量へ含める必要がある。
小さい利用可能高さに合法な切断がない場合は未適合を返し、宣言された最大高さでも
収まらない場合は`Oversize`を返す。宣言高さを超える容量や負値を拒否する。

先頭・連続・末尾の明示改ページは個別に1回消費し、所有ノードを保持する。空の断片でも
改ページを消費した継続位置が進み、末尾の改ページは継続位置がなくてもForcedとして残る。
keepが明示改ページを跨ぐ場合は拒否し、定義末尾のkeepを次の定義へ引き継がない。
候補数上限を超えて探索を打ち切り、残った候補を成功扱いすることはしない。

これは実測された内容断片の選択であり、ページ割当・縦位置・最終脚注予約量・paintの
許可ではない。定義番号のglyphとその占有量の接続、参照を含む本文候補との同時評価、
継続脚注を含むページ状態、構造・公開PDFへの接続はまだ必要である。通常の本文ページ
選択のcostと内部識別子は変更せず、公開profileや公開writerのゲートも解除しない。


### 14.23 脚注定義番号の実font shape

共通flowは定義順の`ProductionFootnoteDefinition`を保持する。番号styleは定義内の
最初の段落・見出しの実computed styleを参照する。入れ子container/list内の段落も対象で、
本文側の参照位置のfontを流用しない。段落がない定義では、classesなしの宣言済み基本
paragraph styleを一度だけ解決して共有する。いずれもfont family/sizeの宣言がなければ
shapeで拒否し、admission先頭fontや仮のfont sizeを補わない。

段落styleを使う場合のlanguageはその段落owner、基本styleの場合は定義ownerの検証済み
language recordへ結び付ける。sourceの定義ID・owner・style参照先・languageを保持し、
flow再構築検証で付け替えを拒否する。

定義番号は、list markerと共用する実font shaperで独立したLTR labelとして組む。
`ProductionFootnoteMarkerShape`は元の定義source、definition index、実glyph/cluster、
実advance、font metricsとgenerated provenanceを保持する。番号を本文bufferへ移したり、
空白へ置き換えたりしない。生成kind・owner-local ordinalもmarker hashへ含める。
markerの保持量はshapeの出力予算に加え、選択済み行の予算へも一度引き継ぐ。

複数桁「10」、定義側だけ異なるfont size、段落なしの実VMB vector定義、別fontでの数字
coverage不足、保持量ちょうど／1不足、style参照先と言語の改変拒否を検査する。
内部識別子はflow `/7`、authored-text-shape `/5`、inline-line-layout `/5`となる。
これはまだ番号列の幅・baseline・上下占有量を脚注frameへ配置した結果ではない。
それらを本文行幅・断片高さへ接続し、参照ページとの同時選択と最終paintを閉じる必要がある。
公開profile・公開writerと全巻受け入れのゲートは維持する。


### 14.24 脚注番号列を除いた内容frame

`ProductionBodyInlineFrames`は、実shape済み定義番号と同じ順序・ownerに結び付いた
`ProductionFootnoteFrame`を保持する。番号列幅は全定義の実advanceの最大値、列と内容の
間隔は全定義番号font sizeの最大値（1 em）とする。複数桁や定義間のsize差があっても
内容の開始位置を揃える。座標は宣言脚注領域を基準にし、本文bodyからの相対位置を保持する。

脚注内容frameの幅から番号列と間隔を差し引き、そのframeを段落・数式・入れ子list等へ
引き継ぐ。段落やcontainer固有のindentはさらに適用する。残り幅が0以下なら、その定義
ownerで`FootnoteFrameExhausted`を返す。定義終了時には元のframeを復元し、次の定義へ
番号列のindentを累積しない。定義ごとの列recordを予算へ加え、列のowner・位置・幅・
間隔・内容frameをfingerprintへ含める。内部body-frames識別子は`/3`となる。

複数桁「10」、異なる番号font sizeの共有列、宣言領域の左右位置、段落indent、定義間の
復元、番号列と間隔だけで幅を使い切る場合を検査する。従来の狭幅fixtureは番号列を
含めた宣言幅へ更新し、断片分割・keep・予算の検証を維持する。

これは横方向の内容幅の確保であり、番号のbaseline・上下占有量・配置を確定した結果
ではない。それらを断片の実測高さへ接続し、参照ページとの同時選択・継続脚注・最終
paintを閉じる必要がある。公開writerおよび全巻受け入れのゲートは維持する。


### 14.25 脚注定義番号のbaselineと上下占有量

実測body flowは、定義番号を各定義内の最初の描画項目に結び付けた
`ProductionFootnoteMarkerBinding`を保持する。定義owner・定義index・定義内item index・
内容上端に対するbaselineを公開するが、任意構築やページ・paintの許可には使わない。
段落行では選択済み実行のbaseline、baselineを持つvector blockではviewport上端offsetと
実baselineの和を使う。baselineを持たないblockやrasterでは番号fontのascenderを使い、
番号の上端を内容上端へ揃える。

番号fontの実ascender/descenderから、内容上端より上の量と内容高さを超える下の量を
算出し、項目のleading/trailingへ反映する。リスト番号と同じ計算を共用し、同じ項目に
複数の番号が付く場合は上下それぞれの最大値を採用する。追加量を合算して同じ領域を
重複予約しない。脚注断片探索の消費高さにこの占有量が含まれる。

定義ごとのbindingを1 recordとして保持予算へ加える。先頭の明示改ページは番号の
描画項目にならず、番号はその項目を実際に含む選択の`marker()`からのみ参照できる。
継続断片には定義番号を繰り返さない。描画項目が全くない定義は、仮の描画項目を補わず
定義ownerの`EmptyFootnote`として拒否する。

段落・実VMB vector・vectorを先頭に持つlist・rasterを使い、baseline、上下追加量の共有、
実消費高さちょうど／1単位不足、先頭改ページと継続時の番号の有無、保持予算の境界を
検査する。これで定義番号を含む内容断片の高さを扱えるが、参照ページとの同時選択、
複数定義間spacing、ページ上の予約・配置・構造とpaintの接続は未完了である。
公開writerと全巻受け入れのゲートは維持する。


### 14.26 脚注参照を実測項目へ結び付ける

`ProductionPreparedBodyFlow`は、検証済み選択行registry内の各参照を借用する
`ProductionFootnoteFlowReference`を保持する。参照元の本文／定義scope、参照先定義、
参照owner、実glyph clusterの先頭・末尾位置は元registryに結び付けたまま、各scope内の
実測itemの先頭・末尾indexへ変換する。改ページやvector等が先行する場合も、段落番号を
そのままitem番号として扱わない。両端が同じ段落と実際の選択行に対応し、指定scope内に
含まれることを照合する。同じ定義への複数の参照ownerを統合しない。

対応付けはsource順の参照と実測項目を一方向に辿り、参照ごとの全巻再走査や段落数に
比例する補助表を作らない。対応recordは参照1件あたり1件として保持予算へ加える。
scopeと両端indexの単調順序を照合したうえで、`references_in_items`は二分探索で
指定item範囲と交差する参照sliceを返す。この読み取り専用query自体は、呼出側が指定した
範囲をページ選択として認可しない。脚注断片の`references()`は、その実選択範囲を使う。
空の強制改ページ断片には参照を付けず、本文の参照を定義の参照と混在させない。

複数行の本文、複数桁「10」、反復参照、先行する改ページ、脚注定義内の相互参照、
vector/listを跨ぐ実item offset、断片ごとの参照の有無、保持予算境界を検査する。
定義内参照は現行tagged profileが`UnsupportedSemantic`で拒否するため、下位の実font・
実vector・実測flowの試験と、公開profileによる拒否の試験を両方保持する。公開ゲートを
解除したり、この試験を公開PDF成功として扱ったりしない。

これは参照を実測候補へ接続するための対応表である。参照先の重複予約防止、依存関係の
ページ状態、本文候補と脚注高さの同時評価、配置・構造と公開paintの接続はまだ必要である。


### 14.27 脚注要求と継続位置の候補状態

`ProductionFootnoteDemandSearch`は、共通の断片探索と同じ保持・work予算を使うownerで
ある。定義ごとに未参照／継続待ち／完了と最初の参照ownerを保持し、最初に要求された
順のqueueで処理する。要求済み・完了済みの定義への反復参照で重複登録や再開始をしない。
queueが空でも未参照定義は残り得るため、これ自体は文書全体の完了判定ではない。

候補状態は不変で、本文の実測item範囲からの要求追加や、選択した脚注断片の消費は
新しい状態を返す。未採用候補を破棄・再評価しても、元の継続位置や要求queueは進まない。
別探索ownerの状態、別候補状態で発行した選択、消費済み位置への選択の付け替えを拒否
する。同じ元状態から別候補を評価することはできるが、保持量・work予算は巻き戻さない。
owner/state識別子はこのin-process検証用であり、PDFや決定的artifactへencodeしない。

`advance`は実選択範囲内の脚注参照だけを要求へ追加する。継続中の定義はqueue先頭に
残り、末尾まで消費した定義は完了となる。定義内の反復・循環参照でも既知の定義を
再登録せず、実際の有限な内容位置だけを進める。ただし定義内参照のtagged profile拒否は
維持し、下位状態の試験を公開PDF対応済みとは扱わない。

状態snapshot・queue・候補・断片の保持量と、状態複製、参照の訪問、queue移動、二分探索
比較数の上界を同じ累積予算へ含める。本文範囲は実items内であることを検査するが、
その範囲のkeep／改ページ規則やページ割当はまだ認可しない。

### 14.28 複数脚注を含む領域内容の選択

`select_region`は候補状態のqueue先頭から、宣言最大高さ以下の容量に実測内容を選択する。
最初の断片のspace-beforeを抑制し、別定義が続く場合は前断片の末尾space-afterと
次定義の先頭space-beforeを加える。末尾だけのspace-afterは消費高さへ加えない。
各断片は領域内offset・番号を含む実消費高さ・内容選択を保持する。

継続切断または明示改ページでその領域を終了する。先頭の空の改ページ断片も1回消費し、
末尾改ページは継続queueが空になっても所有ノードを残す。次定義が残り容量へ収まらない
場合は、それまでの選択と未処理のqueueを返し、元状態は変更しない。最初の断片すら
入らなければ未適合となる。宣言最大高さでの不可分内容の超過は既存のOversize診断を使う。

この領域選択は本文候補と同じページに全要求を置けた証明ではない。本文側のownerが、
新しい参照の脚注を開始できたか、継続を許せるか、bodyとの衝突がないかを同時に評価
する必要がある。物理的なページ原点・構造・paintの認可はまだ発行しない。


### 14.29 全要求の最小断片を確保する脚注領域選択

既存footnote規則に合わせ、`select_required_region`は入口の全pending定義について
合法な最小断片を先に確保してから、残り高さを要求順に配分する。先頭の長い定義だけで
容量を消費し、同じ本文候補内の後続参照の定義を開始できなくする選択を避ける。
`select_region`の部分選択とは区別し、全pending定義を開始できなければ未適合とする。

実測境界の共通kernelに、終端が収まる場合でも前方の合法な切断候補を保持する内部modeを
追加する。keepは硬い制約のままで、候補数・work上限で打ち切った候補集合を成功扱い
しない。通常の本文・単一定義探索のmodeとcostは維持する。

各定義の最小断片と定義間spacingからsuffixの最小必要高さを算出し、後続の最小量が
残る候補だけを評価する。各定義の残容量に合わせて候補を絞り、共通costを再計算する。
同じ領域内で複数定義が継続状態になっても、queue順と各cursorを個別に維持する。
途中定義の明示改ページを消費した後に別定義を同じ領域へ置かず、必要なら改ページの
手前の合法な断片を選ぶ。新たな定義内参照の要求は引き続き次状態へ明示される。

### 14.30 本文境界候補と脚注の同時適合判定

`evaluate_body_candidate`は同じflow内の本文item範囲を受け取り、実行・vector・markerの
高さと項目間余白を計測する。先頭余白と末尾だけの余白を抑制し、keepを切る境界、
内部の明示改ページ、参照の生成cluster範囲を途中で分ける境界を適合扱いにしない。
範囲外のindexは拒否する。ただし前ページからのbody cursor継続はこのAPIでは認可しない。

実際のbody候補の高さと宣言脚注矩形から、重ならない脚注容量を求める。既存の
`FOOTNOTE_SEPARATOR_BAND_RAW`（1 pt）を内容容量から確保し、全要求の最小断片を
確保する領域選択を使う。正の内容がある場合、区切り帯を含む実予約矩形を宣言脚注領域の
下端へ揃える。横方向に離れた領域やbodyより上に収まる領域を不要な衝突扱いにしない。
空の強制改ページだけなら区切り帯の描画矩形を作らず、その改ページ状態を保持する。

結果の`ProductionBodyFootnoteCandidate`は、元の候補状態、実body範囲・高さ、脚注の
実予約矩形、内容選択と次状態へ結び付く。実選択内で新たに発見した脚注参照の定義を
開始できていない場合も未適合とする。元状態や他候補の継続位置は変更しない。

これは局所的な境界と矩形の同時適合である。本文の全候補の比較、連続したページcursor、
ページ数・ページごとの再評価上限、安定性receipt、物理的な全fragment配置と公開paintへの
接続はまだ必要である。既存の公開writer／全巻受け入れゲートは維持する。

### 14.31 本文・脚注の連続ページ探索（実装追補）

`begin_pages`／`select_page`は、本文位置・ページ番号・脚注要求状態を外部から構築できない
`ProductionBodyFootnotePageState`として保持する。同じ探索の状態だけを受け付け、実際に
選択した候補の次状態を、レコードと走査予算を消費するforkで所有する。過去のページ選択を
保持したまま次ページを探索でき、候補の破棄・再評価によって予算を戻さない。

各ページでは共通境界カーネルの合法な本文候補をすべて列挙し、共通本文境界cost、次いで
source順のキーを確定する。その順に§14.30の同時適合を評価し、最初に適合した候補を選ぶ。
残る候補のキーは選択キー以上であり、結果を改善しないため、脚注適合の再評価を省略できる。
この順序付けは比較・移動・保持recordも累積予算に含め、同額キーの元の列挙順を維持する。
候補列挙上限を超えた場合は適合評価前に失敗する。ページごとの評価回数上限は実際に行う
同時適合試行（不適合試行と脚注のみのfallbackを含む）へ適用する。累積work／record上限も
維持し、候補集合を列挙し切れないまま一部だけを最適解としない。
本文が入らず既存の脚注継続がある場合は本文位置を進めない脚注ページも試す。

実選択が到達した本文の強制改ページだけを消費し、先頭・連続・末尾の空ページを保持する。
本文終了後の脚注継続、脚注の強制改ページもページ進行へ反映する。`max_pages`を超える
選択は拒否する。完了状態と未適合は`is_complete`で区別でき、未適合を完了扱いにしない。
keepと強制改ページの矛盾は開始時に拒否する。

この選択は本文境界costに基づく決定的な局所ページ探索であり、全ページを通じた最適性や
脚注分割costの最終policyを認可するものではない。全fragmentの物理配置、ページ列の安定性、
最終receipt、公開writerへの接続、および元全巻・原ノ味・規模・両hostの受け入れは引き続き必要。

### 14.32 選択ページの内容・記号の実座標（実装追補）

`place_page_content`は§14.31の同じ探索が発行したページ選択だけを受け取り、選択結果を
借用する`ProductionBodyFootnotePlacedPage`を生成する。本文と各脚注定義のlocal item番号を
区別し、ページ番号・source・ownerを保持した実fragmentを作る。本文の原点はbody.y、
脚注の原点は実予約矩形.y＋区切り帯＋選択した定義断片のoffsetとする。断片先頭の余白を
抑制し、項目間余白とmarkerによるleading/trailingを保持し、最終消費高を選択時と照合する。

段落baseline、vectorのviewport offset・baseline、raster矩形は通常本文と同じ
`place_flow_item`を使う。独立した数式cursorや推定高さは導入しない。リスト記号は既存の
実shape・frameとmarker bindingから配置する。定義の要求順がsource順と異なっていても、
検証済みの元stream順bindingを二分探索して対応させ、探索workを課金する。脚注番号は
定義の最初の実paint itemの選択時だけ配置し、共有列内で実advanceを右揃えする。基準線と
高さは実フォントのascender/descenderと計測済みbindingを使う。継続ページでは繰り返さない。

配置結果・記号・探索workは同じ累積予算へ課金する。失敗・再配置でも払い戻さない。
これはページ選択に結び付いた内容・記号の物理座標であり、ページ列全体の安定性、数式の
最終terminal、完全なタグ・リンク、区切り帯のpaint、公開writer／manifestの認可ではない。
元全巻・原ノ味・規模・両hostの完了条件は引き続き維持する。

### 14.33 本文・脚注の終端まで連続するページ列（実装追補）

`select_pages`は新しい本文先頭状態から§14.31の実選択を繰り返し、本文の終端・要求済み
脚注の継続終端・必要な末尾空ページの消費を確認してから、所有された
`ProductionBodyFootnotePageSequence`を返す。各ページは直前の選択の次状態からのみ生成する。
本文位置の後退やページ番号の飛び、本文も脚注も進まず必要な空ページも消費しない状態を
拒否する。途中で適合候補を得られない場合は`JointPageNoFit`とし、途中までのページ列を
完成結果として返さない。ページ・候補・record/workの上限は全ページで共有する。

`place_pages_content`は同じ探索が発行した完全なページ列を借用し、全ページの内容・記号の
配置に成功してから`ProductionBodyFootnotePlacedSequence`を返す。選択済みページとgeometryは
借用関係で結び付き、複製による予算のリセットはない。明示された連続／末尾の空ページを
保持する。要求されなかった脚注定義は、組版層では配置しない。ただし未参照定義の公開
tagged-profile admissionは現状`UnsupportedSemantic`であり、低位のnavigation-profile試験で
配置されないことを検証しても、その公開ゲートの成功を意味しない。

この結果は終端までの選択・物理配置を認可するが、複数パス間のページ列安定性や動的参照の
収束、最終math terminal／区切り帯paint／タグ・リンク／公開writer・manifestを認可しない。
元全巻・原ノ味・規模・両hostの受け入れは引き続き必要である。

### 14.34 計測済みflowに対する本文・脚注ページ列の反復一致（実装追補）

`select_stable_pages`は同じ不変の実行・block・脚注計測結果から完全なページ列を2回以上
探索し、両方を実座標へ配置して比較する。一致した場合だけ、最終の所有ページ列とpass数、
発行時の累積record/workを持つ`ProductionBodyFootnoteStablePages`を返す。`max_layout_passes`
が2未満なら拒否する。上限まで一致しなければ`PagePassLimit`であり、未適合・ページ上限・
予算不足も部分結果を発行せず伝播する。各探索・配置・比較で同じ予算を使い続ける。

比較はページ数だけでなく、各ページの本文範囲・実高さ、脚注予約矩形、強制改ページ、
次の本文位置と終端状態、要求順・最初の参照owner・定義ごとの未参照／継続／完了状態と
continuation item位置を含む。脚注の各断片について定義番号・消費範囲・内容数・offset・
容量・高さ・理由・強制改ページowner・候補選択とcostを照合する。実fragmentのsource・
owner・座標・baseline・viewport・余白、リスト記号と脚注番号の実配置も完全一致を要求する。
比較するレコードにもworkを課金する。process内の探索idやsnapshot idは内容の比較対象と
せず、両系列が同じ探索とflowから発行されたことは先に検証する。

これは計測済みの不変flowに対するページ選択・配置の反復一致である。動的なページ番号や
参照文字列を再生成して行を再整形する収束、最終math terminal、区切り帯paint、完全なタグ・
リンク、公開writer／manifest、および元全巻・原ノ味・規模・両hostの完了は別途必要である。

### 14.35 脚注区切り線の実占有矩形（実装追補）

既存footnote-profile painterの寸法を共有定数へ移し、1 ptの予約帯、0.5 ptのstroke、
帯上端から0.25 ptの中心位置を維持する。`ProductionBodyFootnotePlacedPage::separator_ink`
は、実脚注予約矩形の上端・全幅に置く0.5 ptの線の占有矩形を保持する。正の脚注内容がある
ページだけに作り、脚注内の強制改ページだけの選択や空の本文ページでは作らない。
内容のある脚注継続ページには同じ規則を適用する。

区切り線のレコードと生成workも共通予算へ課金し、反復するページ配置の一致比較に含める。
既存painterは数値literalを共通定数へ置き換えるだけで、線幅・中心位置・butt capなどの
描画仕様を変えない。新しい実占有矩形は区切り線の配置情報であり、新しい本文・脚注経路の
公開display/PDF命令、artifact/tag分類、最終PDF receiptへの接続は別途必要である。

### 14.36 安定本文・脚注ページの数式terminal（実装追補）

`finalize_page_math`は同じ探索の安定ページ列と、それを実際に借用する全ページgeometryを
受け取る。別の安定結果や探索のgeometry、異なるlimits／math registry／layout epochは拒否する。
返す`ProductionBodyFootnoteMathTerminals`もそのgeometryを借用し、terminal receipt集合と
式番号の実配置を保持する。式番号のfragment番号はページ順に平坦化した実内容列を指す。

通常本文と同じ`consume_selected_fragment`で、block owner・flow fingerprint、viewport寸法・
位置、baseline、内容高さ、式番号shape・source・幅・高さ・最小間隔を検証する。本文・脚注の
vector blockを実選択順でterminal ledgerへ一度ずつ渡し、ledger全体のfinish／verifyと
式番号総数の一致を要求する。未配置のregistry flowを成功扱いにする例外は追加しない。
inline数式は既存の行配置側の認可が必要であり、このblock terminal処理で代用しない。

ledger・receipt・式番号のrecordsと、走査・検証workは探索の累積予算を使う。terminal用の
JCS／integrity encoderの保守的なピークspool予約を実行前に確保し、同じ探索での再試行でも
加算する。保持するcanonical bytesは別に実測する。このspool会計はterminal段階の累積値で、
全pipelineのspoolや公開出力の原子的publishを認可するものではない。

新しいterminal結果は実選択への結び付きを持つが、公開display/tag/navigation/PDF/manifestの
完成ではない。動的参照の再整形・収束、元全巻・原ノ味・規模・両hostのゲートは引き続き必要。

### 14.37 描画計算の内部入力分離（実装追補）

通常本文専用の所有型に描画計算を固定しないため、内部の`DisplayInput`と`project_display`へ
実行・block・実fragment・list marker・式番号・registry・配置fingerprintを借用する処理を
分離した。型と構築経路はdisplay module内部に閉じ、公開の任意geometry入力は作らない。
既存の`build_production_body_display`は従来のselected layoutを認可入口に保ち、同じ計算を
呼び出す。glyph・inline anchor・vector・raster・generated provenance・予算会計・fingerprint
の生成規則は変更しない。

式番号の検索は、terminalが実選択順に保持した`fragment_index`で行い、parent ownerも
一致することを要求する。脚注の要求順ではparent owner番号の昇順を前提にできないためである。
この分離は共用描画処理の準備であり、本文・脚注terminalからの新しい描画入口、脚注番号・
区切り線・タグ・公開PDFの接続自体は引き続き必要である。

### 14.38 本文・脚注terminalからの共用描画入口（実装追補）

`build_production_footnote_display`は§14.36の検証済みterminal結果を借用し、admissionと
limitsの一致を確認してから`ProductionBodyFootnoteDisplay`を作る。元のregistry、行、block、
geometryへの結び付きを保持し、任意の未検証fragmentを受け取る入口は作らない。

ページ内のfragment番号を全体の実内容列の番号へ変換し、共通の`project_display`で本文・
脚注の文字、inline anchor、inline/block vector、raster、list marker、式番号を生成する。
脚注番号とリスト記号は同じ生成文字描画処理を使い、実フォント・glyph・baseline・advance、
生成buffer key・cluster範囲を保持する。式番号は全体の実fragment番号で検索する。ページ列に
保持された空ページもsource geometryから参照できる。

区切り線はpage番号・実占有矩形・最初の脚注描画の直前位置`before_draw`を持つ別の描画
レコードとして保持する。本文と脚注の描画順を分離せず、番号や記号を含む最初の脚注描画より
前に挿入できる。線を持たない空ページではレコードを作らない。

行・block・terminal fingerprintに加え、実fragmentのsource・owner・定義scope・位置・baseline・
viewport・余白、記号、区切り線を固定長の増分hashへ含める。geometryの大きさに比例する
fingerprint用bufferは作らない。1回の描画生成ではterminalまでのrecord chargeを引き継ぎ、
ページ内コピー、一時projection、保持する描画列と区切り線のrecordsを加算する。別々の描画
生成呼出しを跨ぐpipeline全体の寿命・再試行予算は、このimmutable結果だけでは管理しない。

これは実描画データの生成であり、区切り線のPDF命令化・artifact分類、脚注の構造tag、リンク、
font usage／PDF content／object／manifestの公開接続は引き続き必要である。動的参照の収束、
元全巻・原ノ味・規模・両hostの完了条件を緩和しない。

### 14.39 本文・脚注描画の構造グループ（実装追補）

`build_production_footnote_structure`は認証済みの本文・脚注displayを借用し、同じnavigation／
accessibility authorizationと既存のstructure registry v2を使う。通常本文も共通の
`project_structure`へ接続し、実draw列からページ内のdense MCID、nodeごとのMCR順序、
vector usageを構築する。要求されたpaint nodeの未描画、描画の欠落・ページ順違反、registryに
ない描画は拒否する。生成番号を既存の`FootnoteLabel`へ結び付け、親の`Note`／`Reference`
や定義本文の構造は元のregistryの関係を維持する。

生成文字はslotを区別し、ListLabelなら実list item、FootnoteLabelなら実flowの脚注番号の
生成key・canonical textを使って照合する。文字範囲と実draw文字列を検証し、番号全体の範囲が
先頭から末尾まで欠落・重複なく連続することを要求する。リスト番号と脚注定義番号は1group、
参照番号は必要なら同一ページ内の複数groupへ分かれることを許す。別ページへの番号分割は
認可しない。区切り線は`separator_artifacts`として保持し、MCID／ParentTreeには含めない。

この結果は実displayへの結び付きを持つ構造グループである。新しい経路のPDF marked-content、
ParentTree object、注釈・リンク、font usage／content／manifestへの接続はまだ必要である。
registryの論理的な順序と実PDFの読み上げ順・抽出順は最終の独立検証対象として維持する。
動的参照の収束、元全巻・原ノ味・規模・両hostのゲートを緩和しない。

### 14.40 本文・脚注の構造付き描画からのフォント使用計画（実装追補）

`finalize_production_footnote_fonts`は§14.39の構造結果を借用し、display／admission／limitsを
照合してから実draw列を共通の`finalize_font_projection`へ渡す。通常本文の確定処理も同じ
関数を使う。本文・脚注・参照番号・定義番号・list marker・式番号を、それぞれの実face、
文字範囲、exact text、original glyph IDから使用計画へ集める。非text drawと区切り線には
フォント使用を作らない。描画していないfaceを追加したり、異なるfaceを一つへ置き換えたりしない。

`ProductionFootnoteFontPlans`は元の構造結果に結び付き、draw番号からfont／cluster／CID計画を
引ける。既存のfont finalizerのsubset byte・glyph・Unicode対応とprofile制約を維持する。
生成番号を普通の本文bufferへ読み替えず、displayの生成namespaceを含むtext spanを保つ。

構造までのrecord charge、保持するmath terminal bytesとstructure spoolを引き継ぎ、usage・
文字列／glyph／CID／抽出用コピーとsubset bytesを同じ呼出しの予算へ加算する。全pipelineの
再試行や複数分岐の同時保持を、このimmutable計画だけで認可するものではない。

この段階は使用計画とsubsetの確定であり、新しい経路のPDF text/content命令、marked content、
ParentTree・font object・リンク・manifestの公開接続はまだ必要である。原ノ味の次期profileを
現行CFF経路へ混入しない。動的参照の収束、元全巻・原ノ味・規模・両hostのゲートは維持する。

### 14.41 本文・脚注の実座標とCIDによるPDF文字命令（実装追補）

`encode_production_footnote_text`は構造に結び付いたフォント計画を検証して借用し、
通常本文と共通の文字エンコーダーを使う。本文、脚注、参照番号、定義番号、リスト記号、
式番号の各glyphについて、実配置のx/yと確定済みCIDを使ったTm／Tj命令を生成する。
フォント辞書名は確定済みfont instanceに対応する。非text描画には文字命令を作らない。

結果は元の計画との同一性、draw番号、ページ番号、font instance、各描画のbyte範囲を保持する。
単独描画用のq/Qと必要なcluster ActualTextを含む範囲、および後続のmarked-content処理が
重複したActualTextを作らず包める内部命令範囲を分ける。これはページ全体のroot変換や
構造tagを含む完成PDFではない。

recordsはフォント計画までのchargeに実text描画ごとの保持レコードを加算する。文字命令の
保持bytesは先行spoolを差し引いた容量とmax_output_bytesの両方に制限する。全pipelineの
再試行や複数結果の同時保持の予算管理は、このimmutable contributionだけでは完結しない。

新経路のvector／raster／区切り線を含むページcontent、marked content、ParentTree、
font object、リンク、manifest、公開writerへの接続は引き続き必要である。動的参照の収束、
元全巻・原ノ味・規模・両hostの完了条件は維持する。

### 14.42 本文・脚注vectorのForm確定とPDF使用命令（実装追補）

`finalize_production_footnote_vectors`は認証済みfont計画を借用し、本文と脚注の実draw列を
通常本文と共通のForm確定処理へ渡す。admissionと宣言catalogの一致を保ち、content keyごとに
Formを共有する一方、source occurrenceごとのimage ID、ページ、draw順、描画fingerprintを
保持する。描画しない候補のauditとalias対応は既存のregistry／Form確定処理を維持する。

`build_production_footnote_vector_contribution`は上記計画と§14.41の文字結果が同じfont計画に
結び付くことを検証する。実viewport、変換行列、scale、currentColorを共通PDFエンコーダーへ
渡し、各出現のDo命令、ページ内Form resource、後続tagging用のsemantic hookを作る。
先行font spoolと実text bytesを引いてからvector命令を確定する。任意の小さい先行spool値を
この入口の引数で指定することはできない。

### 14.43 本文・脚注の統合ページcontentと区切り線Artifact（実装追補）

`build_production_footnote_page_content`は上記の文字・vector処理と共通raster確定処理を
順に実行し、本文と脚注のdraw順を保ったページcontentを作る。rasterは実選択画像から
PNG decode／Flate／alpha mask、または認証済みJPEGを使い、内容hashでpayloadを共有する。
実viewportの幅・高さ・原点から描画行列を作り、figureやcaptionのsource関係を保持する。

ページ冒頭のY反転は1回とし、強制改ページで選択された空ページも残す。脚注区切り線は
実ink矩形から0.5ptの黒いbutt線を作り、最初の脚注drawの直前に独立した
`/Artifact BMC ... EMC`として置く。MCIDは付けない。各ページは通常draw範囲とは別に
Artifactのbyte範囲と挿入先draw番号を保持し、後続marked-content処理が再配置せず取り込める。

通常本文も同じページ合成処理を使い、そのArtifact列は空になる。draw／text／vectorの
未消費、ページの逆行、区切り線の未消費を拒否する。結果は実font計画との同一性を維持する。
先行record計画、text／vectorの保持bytes、rasterの保持bytesとPNG一時workspaceを引き継ぎ、
ページcontentの保持bytesをspoolとoutput上限へ加算する。既存vector計画が持つIR／canonical
表現などを含めた全allocationの寿命管理、再試行・分岐の横断予算は引き続き別の完了要件である。

この段階は統合ページcontentであり、構造groupからのmarked content、ParentTree・font／image
objectの最終統合、注釈・リンク・manifest・公開writerの接続はまだ必要である。
動的参照の収束、元全巻・原ノ味・規模・両hostの完了条件を緩和しない。

### 14.44 本文・脚注の構造付きページ命令（実装追補）

`build_production_footnote_marked_content`は§14.43のページcontentを検証して借用し、
そのfont計画が保持する同一の構造結果を使う。通常本文と共通のmarked-content生成処理で、
実groupごとのrole、ページ内MCID、Langを付ける。文字のActualTextは実groupに属する
draw文字列だけを連結し、元source nodeの全文やcluster単位の置換を重ねない。
数式の意味文字列は既存の非描画anchorを使い、実viewportとbaselineを保持する。

区切り線Artifactは元ページのbyte範囲をそのまま、対象groupの直前かつ全MCID／ActualText
scopeの外に取り込む。groupの途中を分断するArtifact、未消費のArtifact、draw順の不一致、
未消費のgroupを拒否する。強制改ページで残った空ページも保持する。

脚注経路では構造がfont確定より前にあるため、そのrecords／spoolはページcontentへすでに
含まれる。通常本文の並列branchを合流する加算を再適用せず、ページ結果・数式anchorの
recordsと新しいmarked bytesを加算する。生成結果は元contentとの同一性を検証できる。

この段階はMCIDを含むページ命令であり、構造object／ParentTree、font・image object、
脚注参照先を含む注釈・リンク、manifest・公開writerの最終接続は引き続き必要である。
独立した完成PDFの抽出順・tag・navigation検証、および動的参照、全allocation寿命管理、
元全巻・原ノ味・規模・両hostの完了条件を維持する。

### 14.45 脚注参照番号から実定義へのnavigation計画（実装追補）

`build_production_footnote_reference_navigation`は構造結果とadmission／limitsを検証し、
同じterminal ownerが保持する脚注行の定義・参照対応を使う。生成FootnoteLabelの親を
NoteまたはReferenceとして照合し、通常のsource anchorやURIとは区別した脚注専用計画を作る。

定義のdestination列はsource定義順を維持する。移動先のページ・fragmentは最初に実配置された
定義番号と一致し、領域はその番号と最初の実内容fragment、および同fragmentの全描画を含む。
数式・図・大きいlist markerで始まる脚注でも上端を取りこぼさない。継続fragmentを新たな
定義先として登録しない。

参照link列は実ページ／描画順と各Reference構造nodeを保持し、参照番号の実logical boundsを
同じfragment内で結合する。複数回の参照を失わず、それぞれがsourceに対応した定義indexを指す。
ページごとのlink範囲を保持し、未配置の定義／参照、二重の定義ラベル、順序不整合を拒否する。
任意の座標や参照先を入力する公開constructorは作らない。

先行marked-contentなどを保持するrecord chargeを受け取り、構造より小さい基準値は拒否する。
定義・参照・最初のfragmentの一時index、保持結果、ページ範囲と各linkを同じ上限へ加算する。
結果は元の構造ownerとの同一性、admission／limitsを再検証できる。

これは脚注専用の幾何・参照先計画である。通常anchor／URI／outlineを含むjoint navigation、
注釈object・StructParent、定義destinationのPDF化、必要な戻りリンクと最終writerへの統合は
引き続き必要である。元全巻・原ノ味・両hostを含む完成PDFの受入条件を緩和しない。

### 14.46 通常navigationと脚注参照計画の共用入口（実装追補）

`build_production_footnote_navigation`は同じ構造結果の実draw、inline anchor、全ページの
fragment列から通常anchor・internal／URI link・outlineを確定する。通常本文も共通の
`project_navigation`を使い、source宣言・構造nodeとの照合、Linkの継承と矩形結合、
anchorの最初の実配置、outlineの親子・兄弟関係を維持する。脚注内のanchorと通常リンクも
実脚注fragmentから位置を得る。URI文字列を作り直したり、sourceの参照先を置換したりしない。

全ページのfragmentを描画順のiteratorで渡し、通常navigationのためだけの全fragmentコピーは
作らない。既存のページ内link範囲、nodeごとのlink index、source anchor名とoutline entryを
新しい結果からも参照できる。

通常navigationのrecordsを先行保持量へ加算してから§14.45の脚注参照計画を確定する。
共通の構造・displayを二重計上せず、両計画を保持した追加recordsと元の基準値を公開する。
両計画は同じ構造結果に結び付き、統合fingerprintはそれぞれの結果のfingerprintを含む。
通常anchorのindexと脚注定義indexを混同しないよう、脚注参照計画は型を分けて保持する。

これはnavigation計画の統合である。両種のリンクをPDF注釈順へ統合する処理、
StructParent・destination・outline objectの最終構築、必要な戻りリンク、公開writerへの接続は
引き続き必要である。完成PDFの独立検証、動的参照の収束、全allocation寿命管理、
元全巻・原ノ味・規模・両hostのゲートを維持する。

### 14.47 通常リンク・脚注リンクのPDF注釈object（実装追補）

`build_production_footnote_annotations`は構造付きページ結果を検証し、同じ結果から
navigationを確定する。通常リンクと脚注参照を実ページ、fragment、source owner、種別内indexの
順に統合し、連続したLinkAnnotation roleを割り当てる。各注釈は元の種別・index、
構造node、ページ、StructParent keyを保持し、ページごとの範囲とnodeごとの注釈indexを作る。

Rectは選択されたlogical boundsをページのPDF座標へ変換する。通常内部リンクは元の
anchor名をUTF-16BEで使い、URI actionは検証済みURIの元バイトを保持する。脚注リンクは
実定義ページと最初の内容領域への直接XYZ destinationを使い、通常anchor名のnamespaceへ
合成名を混入しない。Contentsは通常Linkのaccessible nameまたはsourceの脚注番号を使う。

StructParentはページ数に注釈indexを加えたkeyとし、ページMCID用のkeyと分離する。
この段階は未解決Page参照を含むobject contributionであり、ParentTree・OBJR・ページAnnotsを
実際に発行する最終ownerが同じbindingを使う必要がある。注釈だけのobject上限に加え、
全objectを統合した最終上限検証も維持する。

先行marked-contentとnavigationのrecords／spoolに、一時整列列、親Linkの索引、
保持binding、node・page索引、object/chunkとdictionary bytesを加算する。親Linkの検査は
parent-before-childの構造列を一度たどる。脚注番号が既存Link内にあり異なるdestinationの
注釈が重なる意味構造は既存tagged-profileが拒否し、注釈側でも防御的に拒否する。

ParentTree・最終ページ／font／image object、通常destination／outline、必要な戻りリンク、
manifest・公開writerへの統合はまだ必要である。完成PDFの独立検証、動的参照、
全allocation寿命管理、元全巻・原ノ味・規模・両hostのゲートを維持する。

### 14.48 本文・脚注のParentTreeと構造object（実装追補）

`build_production_footnote_structure_objects`は§14.47の注釈結果を検証して借用し、
通常本文と共通の構造object生成処理を使う。StructTreeRoot、ParentTree、全構造node、
必要なIDTreeを生成する。ページ内の実MCID順でParentTreeのページ配列を作り、
続けて注釈のStructParent keyを対応するLink／Reference nodeへ結び付ける。

各nodeのKには実groupのMCR、registryにある子node、そのnodeに属する注釈のOBJRを保持する。
OBJRは同じ注釈bindingのページとLinkAnnotation roleを参照し、別nodeへの対応は拒否する。
Note／Reference／生成Labelの関係、Lang・Alt・ID・list/table属性は元registryから引き継ぐ。
区切り線Artifactは構造nodeやParentTreeの要素に加えない。

生成前に、保持済み注釈object数と今回必要な全構造object数を合算して上限を確認する。
records／spoolも注釈結果までの保持量を引き継ぐ。生成結果は元注釈ownerとの同一性を
検証でき、既存objectを無償で複製したり、独立した新予算へ切り替えたりしない。

この段階ではPage参照は未解決である。最終ページのAnnots／StructParents、font・image・
vector・content object、通常destination／outlineと全object番号の統合、必要な戻りリンク、
manifest・公開writerへの接続はまだ必要である。完成PDFの独立検証、動的参照、
全allocation寿命管理、元全巻・原ノ味・規模・両hostのゲートを維持する。

### 14.49 本文・脚注のfont／media／ページresourceとnavigation object（実装追補）

`build_production_footnote_resource_objects`は§14.48の構造object結果を検証して借用し、
通常本文と共通のresource生成処理へ接続する。確定済みfont program・ToUnicode・CID対応、
数式の非描画anchor font、vector Form／ExtGState、raster／mask、構造付きPageContentと
各ページのPageResourcesを生成する。実ページで使ったfont／vector／rasterだけをそのページの
辞書へ結び付け、空ページを落とさない。選択済みfontや画像を代用品へ置換しない。

通常destinationとoutlineも共通のobject生成処理を使う。destination名はPDFのUTF-16BE
name-tree順に並べ、実ページと座標を参照する。outlineの親子・兄弟roleと元のdestination名を
維持する。resource内参照とoutline内参照はこのcontribution内で閉じることを確認し、
Page参照は範囲を検証して最終page-tree ownerへ残す。

Builderは先行object数を保持し、新しいobjectを追加するたびに注釈・構造・resourceの合計で
上限を検査する。records／spoolも構造object結果までの値を引き継ぐ。すべての保持object数を
結果から参照でき、既存contributionのobjectsを複製せずに次の統合段階へ渡せる。

これはPageResourcesと各contributionの確定であり、最終PageのAnnots／StructParents、
page tree・catalog・metadata、絶対object番号・xref、manifest・公開writerの統合はまだ必要である。
既存TT／TTC／CFF1の小fixture検証を原ノ味・次期CFF profileの受入証拠に読み替えない。
動的参照、全allocation寿命管理、元全巻・原ノ味・規模・両hostのゲートを維持する。

### 14.50 本文・脚注の最終object番号と診断PDF組み立て（実装追補）

`assemble_production_footnote_pdf`はresource結果から注釈・構造・描画までの所有関係を検証し、
借用iteratorで各contributionを通常本文と共通のPDF組み立て処理へ渡す。全objectを事前に
複製する配列は追加しない。Pageを含む全object数と累積recordsを番号割当て前に検査し、
重複role・解決できない参照・予定数と実object数の不一致を拒否する。

Catalog、Pages、元メタデータ、PageのMediaBox／Contents／Resources、StructParentsと
本文・脚注を統合したAnnotsを生成する。構造・注釈・destination／outline・resourceを同じ
絶対番号へ解決し、objectのoffset・長さ・hash、xref、trailerを確定する。spoolは先行
contributionと組み立て中のコピーを累積し、最終bytesの出力上限も検査する。結果は元の
resource ownerを借用し、別ownerによる検証を拒否する。

16種の小fixtureで実objectのhash／offset／xref、ページ参照、注釈順序、空ページ、再実行の
byte一致を検証する。records・spool・全object数・出力bytesは境界値と1不足の拒否を検証する。
これは独立PDFツールで検査できる診断assemblyであり、`VerifiedPdfBytesReceipt`や公開可能性の
証明ではない。manifest・公開writer・CLIへの接続、必要な戻りリンク、動的参照、全allocation／
retry管理、元全巻・原ノ味・規模・両host受入は引き続き必須である。

### 14.51 実入力から本文・脚注PDFまでの共通owner（実装追補）

`with_production_common_footnote_pdf`は検証済みpackage・navigation・semantics・profile・
resource ledgerからsource flow、vector binding、数式flow、block layoutを用意し、共通の
行再形成を安定化させる。その実結果の本文行と脚注行でPreparedBodyFlowを作り、本文・脚注の
全ページ選択と配置の一致を確認する。確定した実ページに対して数式terminalを発行・検証し、
§14.38–14.50のdisplay、structure、font、content、marked content、注釈、構造object、
resource、PDF assemblyを同じ所有寿命の中で接続する。テスト用の既成layoutを引数に受け取らない。

行の候補探索で消費したstepsを呼出し全体の探索予算から差し引き、残りだけを脚注需要・
ページ選択と配置へ渡す。観測値は実行した行／ページpass数、両探索の消費量、最終records、
displayと数式registryのhash、実block数式terminal数を返す。完全に成功した場合だけ
借用PDFと安定ページ結果をcallbackへ渡す。予算不足、ページ数超過、配置不能ではcallbackを
呼ばず、部分PDFを成功として返さない。

通常navigation、脚注内数式・式番号、複数ページ継続脚注で再生成bytesと観測値の一致を検証する。
探索予算の境界値と1不足、ページ上限・配置不能を実入力から検証する。このownerの追加だけでは
公開writerのreceipt／manifestやCLIの診断code体系は閉じない。動的ページ参照と行の再帰的な
収束、全allocation／retry寿命、元全巻・原ノ味・規模・両hostの受入要件を残す。

### 14.52 統合PDFのvector最終writer観測値（実装追補）

本文・脚注のPDF assemblyは、実際に割り当てたobject番号表から既存の
`StagingSafeVectorPdfFinalWriterObservationV2`を生成する。relative vector roleの絶対番号、
各usageのPage／PageContent／Form番号、元のusage ID・page・paint ordinal・contribution
fingerprintを保持する。番号を別の予測表で作らず、未解決roleがあればassemblyを拒否する。
既存contribution検証器でobjectとusageの過不足、番号衝突、ページ対応も検査する。

行の保持と検証用map／setの記録数はassemblyまでの累積recordsに追加し、確保前に上限検査する。
観測JSONは同一のエンコーダーを計数sinkで一度実行し、残spool以内と確認してから正確な長さを
予約して生成する。その実長を累積spoolへ加える。既存の公開constructorとJSON形式は維持し、
新しい内部constructorは使用可能spoolを受け取る。

16種の統合fixtureでcontributionとの各行対応、実PDF番号、描画fingerprint、既存constructor
との観測値一致を検証する。既存のassembly境界テストはこの追加records／JSONも含める。
これはvector closureへ接続するための最終番号観測であり、PDF receipt・book／tagged
manifest全体の成立や公開writerの置換を証明するものではない。元全巻等の受入ゲートを維持する。

### 14.53 実ページ・リンク・言語からbook navigation入力を構成（実装追補）

`project_production_footnote_book_navigation`は本文・脚注navigationとその構造／displayの
所有関係を検証して借用し、book選択に必要な実ページ寸法、destinationのsource owner・
fragment・座標、通常内部リンクの実矩形を構成する。リンクは既存book契約のpage／owner／
座標順へ並べ、複数行・複数ページの各矩形を保持する。URIと脚注参照は専用注釈に残し、
通常の内部destinationと混同しない。既存のpage・destination・link検証器を共有する。

通常本文の非既定言語paintは実drawと論理ownerのoccurrenceから作る。vector言語paintは
全usageの実draw ordinal・source owner・kind・言語record fingerprint・draw fingerprintを
保持する。式番号は通常ownerと異なる言語child recordへ結び付け、親言語fingerprintと
全childの実paint被覆を検証して別のchild paint列で保持する。既存言語検証器が通常ownerと
vectorの被覆を確認する。旧Display receiptを仮生成しない。

保持行と検証用入力／map／setのrecord予算を確保前に検査し、複製するanchor名・リンク名・
言語文字列を累積spoolへ加算する。結果は別navigation ownerによる検証を拒否する。
共通source-to-PDF driverは最終assemblyの実records／spoolを引き継いでこの入力を作り、
成功callbackの中でPDF・安定ページ・book入力を同じ寿命で提供する。

これは既存book receipt／PDF観測／manifestへ実情報を渡す入力境界であり、それらの封印や
公開出力はまだ完了していない。動的参照、全allocation寿命、元全巻・原ノ味・規模・両hostの
必須ゲートを維持する。

### 14.54 実book選択receiptと容量付きcanonical化（実装追補）

`seal_production_footnote_book_navigation`は§14.53の入力を消費し、検証したsource navigationと
profile・limitsに結び付いた`ProductionFootnoteBookNavigation`を作る。内部の
`BookNavigationSelectedReceiptV2`は既存のcanonical形式を保持する。ページ・destination・
リンク・通常／vector言語の行を複製せず移動し、outlineは既存resolverで実destinationへ解決する。
selected fragment数は実配置fragmentの合計、layout／display hashは実joint displayから得る。
式番号の子言語paintは外側のownerで保持し、通常の言語ownerへ偽装しない。

入力までのrecordsにoutlineの保持行・文字列とdestination検証map／setを追加する。
outlineが複製する文字列も事前にspoolへ加える。destination registry、outline entries、
通常言語、内部リンク、ページ、vector言語および選択全体のJSONは、共通emitterを計数sinkと
文字列sinkで実行する。各必要長を確保前に検査し、hash用の一時JSONも累積spoolへ計上する。
旧選択経路のエンコードも同じemitterを使い、JCSの文字escaping・field順・hash形式を維持する。

共通source-to-PDF driverはこのreceiptまで生成し、PDF・安定ページと同じcallback寿命で渡す。
別navigationや別profileは拒否する。旧Displayの強い検証を代用せず、新しいwrapperのsource
検証と既存のsealed内容検証を区別する。これで公開PDF receiptやbook／tagged manifestが
成立したとは扱わない。公開writer／CLI、動的参照、全allocation寿命と元全巻等のゲートを残す。

### 14.55 実PDFのbook最終writer観測（実装追補）

`observe_production_footnote_book_pdf`は同じnavigationに結び付いたPDF assemblyとbook選択を
検証し、実Catalog／Info object bytes、Metadata stream内の実XMP、outlineの絶対番号・
親番号、通常／vector言語paintの実PageContent番号から既存のbook最終writer観測を構成する。
式番号の子言語paintは別列で保持する。言語paintのdrawが所属する実marked-content groupの
ページ・言語とも照合する。別navigationで作ったbook選択を組み合わせることは拒否する。

assemblyは確定したoffsetと長さからobject本体の借用sliceを定数時間で返せる。観測のために
PDF全体やobject本体を複製しない。Info・outline・言語の保持行／文字列と検証mapを累積予算へ
加算する。既存book観測エンコーダーを共通emitterにし、Info・言語・outlineの一時JSONと最終
観測JSONは必要長を計数してから確保し、すべてspoolへ計上する。公開constructorと形式は維持する。

共通driverはPDF・安定ページ・book選択と、この最終writer観測までを同時に提供する。
この観測は公開権限を発行しない。XMPは実assemblyの適合宣言なしのbytesを記録する。
公開book closureが要求する適合宣言付きXMPとは異なり、その検証は引き続き拒否する。
PDF receipt、公開book／tagged manifestとwriter／CLI、全巻等の受入は未完了である。

### 14.56 実ParentTree payloadの検証（実装追補）

本文・脚注PDF assemblyの`verify`はsource chainの照合に加え、確定したParentTree objectの
実payloadを構造groupと注釈bindingに照合する。各ページのMCIDが0始まりの連番であること、
groupのページ、各MCIDの実StructElem番号、ページ数に続く注釈キー、注釈の所属ページと
実StructElem番号を確認する。空ページと同じ構造要素を参照する複数MCIDも保持する。

計数値をstack上でformatしながら借用sliceの先頭と比較するため、検証用の対応表や文字列を
新たに確保しない。欠落・変更・余分な末尾をすべて拒否する。この検証はbook最終writer観測が
assemblyを検証する経路でも実行される。公開tagged観測・PDF/UA適合性・release receiptの
成立を意味せず、構造dictionary全体・MCID stream・OBJRの最終closureは引き続き必要である。

ローカル検証: `cargo test -p typaxis-pdf production_parent_tree --lib`は1件成功。
`cargo test -p typaxis-cli production_footnote_page_content_combines_draws_and_separator_artifacts`
は18種の実PDFを含む1件成功。`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`
は163件成功・1件ignored（13.02秒）。いずれも`workspace/Cargo.toml`と
`CARGO_TARGET_DIR=/private/tmp/typaxis-vmb-book-build`を使用した。

### 14.57 実構造root・StructElem・MCR・OBJRの検証（実装追補）

本文・脚注assemblyの`verify`は実StructTreeRootのRoleMap、ParentTree参照、次の注釈キー、
root node列とIDTree参照を照合する。各StructElemは実object bytes全体を検証し、role、親参照、
言語、Alt、ID、リスト／表属性、子の順序をsource registryと照合する。MCRはnodeごとの実groupから
ページ絶対番号とMCIDを確認し、OBJRはnodeごとの注釈bindingからページ・注釈絶対番号を確認する。
両方のnode別indexについて所有者も再確認し、未定義のobject参照を拒否する。

文字列はUTF-16 code unitをstack上でhex化して実sliceと直接比較し、検証用の文字列や配列を
確保しない。日本語、surrogate pair、PDF delimiter、改行、空文字を明示したunit testを追加した。
ParentTreeと合わせて双方向の構造参照を照合するが、IDTree本体・marked streamの最終検証、
公開tagged観測／PDF receipt／manifestおよび元全巻等の必須受入ゲートは未完了である。

ローカル検証: `cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（12.60秒）。root検証追加後、`cargo test -p typaxis-pdf --lib
production_body_assembly::production_parent_tree`は2件成功、`cargo test -p typaxis-cli --bin typaxis
production_footnote_page_content_combines_draws_and_separator_artifacts`は18種のPDFを含む1件成功
（4.06秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.58 実IDTreeの完全対応検証（実装追補）

本文・脚注assemblyの構造検証はIDTree本体も照合する。実payload中の絶対object番号から
実object観測のStructureNode roleを解決し、そのregistry nodeが持つ構造IDと項目全体を比較する。
IDは既存writerと同じsource byte順で厳密な昇順を要求し、項目数をregistryのID数に一致させる。
これにより重複・欠落・異なるIDとobjectの組合せを拒否する。IDがない場合はIDTreeが存在しない
ことも要求する。参照番号の先頭0、桁あふれ、異なるgeneration、余分な末尾も拒否する。

照合は実sliceの逐次読み取りとobject／registryの定数時間参照で行い、ID用map・sort用配列・
複製文字列を作らない。unit testはUnicode ID、重複・順序逆転、件数不一致、全byteの変更と
全位置の切り詰め、数値境界を検証する。実脚注fixtureでは生成したdefinition IDに接続する。
これは構造object検証の拡張であり、marked stream・公開receipt／manifest・全巻等の受入は
引き続き未完了である。

ローカル検証: `cargo test -p typaxis-pdf --lib production_body_assembly::production_parent_tree`
は3件成功。`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（12.68秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.59 実PageContent streamと選択marked contentの一致検証（実装追補）

本文・脚注assemblyの`verify`は、各実PageContent objectのdirect Length、stream開始／終了の
形式、payload全体をsource chainで保持するmarked pageと照合する。ページ数とpage indexの
連続性も再確認する。これにより構造groupから生成済みのMCID・ActualText・言語・本文／脚注
描画・separator artifactを含むbytesが、実PDFの対応するPageContent objectに保持されたことを
確認する。生成器とは独立したPDF命令parserやPDF/UA検証器の代替にはしない。

借用sliceを直接比較し、stream全体の再確保・UTF-8変換・keyword検索は行わない。unit testは
payload中の`endstream`等の文字列、非UTF-8 bytes、空payload、全byteの変更と全位置の
切り詰め、Length不一致、末尾追加を検証する。公開receipt／manifestへの封印、最終ページ／
resource参照の検証、公開writer・CLIと全巻等の受入ゲートは引き続き未完了である。

ローカル検証: `cargo test -p typaxis-pdf --lib production_body_assembly::production_parent_tree`
は4件成功。`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（34.53秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.60 実ページツリーとContent・Resources・注釈参照の検証（実装追補）

本文・脚注assemblyの`verify`は実Pages objectのCount・Kids順序と、各Page object全体を
照合する。親Pages、実MediaBox、PageResources／PageContentの絶対参照、StructParentsキー、
Tabs設定、Annots列をsource geometryと注釈bindingに結び付ける。注釈rangeがページ順で連続し、
逆転せず、全bindingを一度ずつ覆うことと、各注釈の所属ページも検証する。

寸法は固定小数点整数をstack上で十進表現にし、既存writerと同じ正確な値を直接比較する。
負値・整数・小数最小単位・i64両端について旧writerとの一致を確認し、代表値には明示した
期待文字列も使う。検証用の新規文字列／配列を作らない。Resources dictionary内の全参照、
公開receipt／manifest・writer／CLIへの接続と全巻等の受入ゲートは引き続き未完了である。

ローカル検証: `cargo test -p typaxis-pdf --lib production_body_assembly::production_parent_tree`
は5件成功。`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（53.72秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.61 全contribution objectと実参照の一致検証（実装追補）

本文・脚注assemblyの`verify`は注釈・構造・resourceの全contributionを元の割り当て順にたどり、
実object観測の番号・role、role番号map、実payloadを照合する。Bytes chunkはそのまま比較し、
Reference chunkだけを絶対番号へ解決する。参照先の実object観測にも同じroleと番号があることを
要求する。検証後のobject総数とretained contribution数も一致させ、欠落・余分なobjectを拒否する。

この経路でPageResources内のfont／image／Form参照、font program／ToUnicode、vector／raster、
注釈・destination・outline等の出力bytesをsource chainの保持内容と照合する。既存の構造registry、
page geometry、marked contentに対する個別検証も維持する。新たなobject graphや複製文字列は
確保しない。unit testは型付き参照のみの置換、参照不在・0・誤番号、raw bytes中の参照に似た文字列、
非UTF-8、全byte変更・全位置切り詰め・末尾追加を検証する。これは公開適合性の発行ではなく、
公開receipt／manifest・writer／CLIと元全巻等の受入ゲートは引き続き未完了である。

ローカル検証: `cargo test -p typaxis-pdf --lib production_body_assembly::production_parent_tree`
は6件成功。`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（56.43秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.62 最終PDF envelope・xref・hashの検証（実装追補）

本文・脚注assemblyの`verify`は、実PDF全体のheader、連番objectの開始位置・長さ・終了形式、
各payloadのSHA-256、traditional xrefの全offsetとgeneration、trailerのSize／Root／Info、
startxref、EOFを実bytesと照合する。最後に全体SHA-256を再計算し、保持済みの最終hashと一致
させる。xref offsetは10桁の範囲を要求し、余分な末尾も拒否する。source object照合と併用し、
観測だけが整合した別のbytesを受け入れない。

PDF全体を複製せず借用sliceを順に消費する。unit testは全byte変更・全位置切り詰めに対して
改変後の全体hashを与えても拒否すること、offset／長さ／番号／payload hash／全体hashの変更、
object欠落・末尾追加を検証する。fixtureはenvelopeのみを検証するもので適合PDFの受入証拠には
しない。公開receipt／manifest・writer／CLIへの接続と元全巻等の受入ゲートは引き続き未完了である。

ローカル検証: `cargo test -p typaxis-pdf --lib production_body_assembly::production_parent_tree`
は7件成功。`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（36.40秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.63 実Catalogのsource navigation照合（実装追補）

公開接続条件の照合では、既存book closureが適合宣言付きXMPを要求し、既存tagged観測が成立済み
book／safe-vector closureに依存することを再確認した。適合宣言なしのjoint assemblyをそのまま
公開receiptへ変換せず、必要な実PDF検証を継続する。

本文・脚注assemblyの`verify`は実Catalog全体も照合する。Pages・Metadata参照、sourceのdocument
language、MarkInfo・ViewerPreferences、実StructTreeRoot参照、destination／outlineの有無と
絶対参照を確認する。source navigationにdestination／outlineがない場合は対応するobject roleも
存在しないことを要求する。UTF-16言語文字列と各参照は借用sliceへ直接照合し、複製文字列を作らない。
公開receipt／manifest・writer／CLIおよび元全巻等の受入ゲートは引き続き未完了である。

ローカル検証: `cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は163件成功・
1件ignored（12.30秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.64 共通本文・脚注driverのPDF予算診断（実装追補）

共通driverのmarked content、注釈／構造／resource object、assembly、book最終writer観測の
型付きエラーを分類し、予算超過をInternalへ潰さずFailureKind::Limit（exit 5）へ渡す。
object数／PDF allocationはG6100、記録数はL5110、output／spoolはD8101としてstageと元errorを
保持する。navigation内の記録数／allocation超過もLimitへ分類する。receipt／font／structure
不一致等の内部整合性エラーはI9190のInternalとする。

実source-to-PDF driverで成功した値からobject数・output bytes・spool・記録数を各1だけ減らし、
Limit分類・診断prefix・未完成PDFをcallbackへ渡さないことを確認する。display／font／content等の
他stageと配置失敗の精密診断、公開writer／manifest・CLIへの接続および全巻受入は引き続き必要である。

ローカル検証: `cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`では既存163件が
成功。追加テストの記録数診断の期待値をD8101からL5110へ修正後、`cargo test -p typaxis-cli
--bin typaxis production_common_footnote_pdf_limits`が成功（1.10秒）。変更後の4予算境界と
callback非公開を確認した。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.65 共通本文・脚注driverの配置失敗診断（実装追補）

共通driverのprepared flow、脚注demand search、安定ページ選択、実配置、数式terminal確定から
返るProductionBodyPaginationErrorを型で分類する。ページ数・ページpass・探索work・lookback・
fragment・allocation上限はL5110／Limit、spoolはD8101／Limitとする。JointPageNoFit、oversize、
不正な脚注容量、強制改ページを越えるkeepはL5100／Input、receipt／幅の不一致・算術overflowは
I9190／Internalとして区別する。元errorのkindとowner node番号を保持し、数式terminalの内側の
診断分類も引き継ぐ。本文行探索後に共有candidate残量が不足する場合もLimitを返す。

実source-to-PDF testは探索workの1不足、max_pages=1、脚注領域が小さすぎる入力を使い、
それぞれLimit／Limit／Inputとprefix、途中PDFをcallbackへ渡さないことを検証する。
未実装regionや内容の空要素は入力エラーとして維持する。他stageの診断、公開writer／manifest・
CLIへの接続と全巻受入ゲートは引き続き必要である。

ローカル検証: `cargo test -p typaxis-cli --bin typaxis production_common_footnote`は3件成功。
`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は164件成功・1件ignored
（14.81秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.66 共通行再組版の内側の予算診断（実装追補）

共通本文・脚注driverはProductionBodyReshapeErrorを文字列prefixで分類せず、Shape／Layout／
Feedbackの内側の型を確認する。inline candidate・selection・unit、shape context・出力record、
再組版iteration・line shape・paragraph textの上限とallocation失敗をL5110／Limitへ渡す。
shapeのOutputLimitはbytesではなくmax_fragmentsによる保持record数の上限であることを確認し、
D8101と混同しない。receiptやline context等の内部不一致と、実行可能な改行がない入力を区別し、
元の内側のerrorをmessageに保持する。

4種の実source-to-PDF fixtureで、共有探索予算0と成功必要量の1不足の両方がL5110／Limitに
なること、必要量ちょうどでは同一PDFを返すことを検証する。その他のresource内部診断、
公開writer／manifest・CLIへの接続と全巻受入ゲートは引き続き必要である。

ローカル検証: `cargo test -p typaxis-cli --bin typaxis production_common_footnote`は3件成功。
`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は164件成功・1件ignored
（12.07秒）。shape OutputLimitの分類修正後、実max_fragments=5／6境界を使う
`production_authored_text_charges_output_across_paragraphs`が成功（0.10秒）し、L5110／Limitと
元node 5の保持を確認した。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.67 共通display・structureの予算診断（実装追補）

共通本文・脚注driverのdisplay／structure生成は型付きエラーを分類する。記録数・allocation上限は
L5110／Limit、structureのspool超過はD8101／Limitとする。receipt・registry・paint不一致や
算術overflowはI9190／Internalとして区別し、displayのowner nodeも保持する。未対応の式番号は
入力エラーとして維持する。

実脚注displayの記録数1不足でL5110とownerを検証する。structureでは記録数とspoolの各必要量
ちょうど／1不足を確認し、別displayによるreceipt不一致がI9190／Internalとなることも検証する。
font／content内部の診断、公開writer／manifest・CLIへの接続と全巻受入ゲートは引き続き必要である。

ローカル検証: `cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は164件成功・
1件ignored（12.28秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.68 共通page content内部の予算診断（実装追補）

共通本文・脚注driverはProductionBodyPageErrorのText／Forms／Vectors／Rastersを型で分類し、
一律Internalへ変換しない。text記録数はL5110、content／textのoutput・allocationと既存Form／
vectorの予算分類はD8101、raster ResourceLimitはG6100のLimitとする。receipt・candidate・
placement等の内部不一致はI9190／Internalに保持し、内側のerror全体をmessageへ残す。

実text encodingの記録数／spool／output境界、実Form planの記録数不足、実vector contributionの
spool不足、実page contentのresource記録数／spool／output境界で診断分類を検証する。
font内部の精密診断、公開writer／manifest・CLIへの接続と全巻受入ゲートは引き続き必要である。

ローカル検証: 追加assertionの挿入位置を修正した後、`cargo test -p typaxis-cli --bin typaxis
production_ -- --skip 5000`は164件成功・1件ignored（16.63秒）。
`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.69 共通font確定の予算・入力・整合性診断（実装追補）

共通本文・脚注driverのfont確定ではResourceLimitをG6100／Limitにし、一律Inputへ変換しない。
CFFのtable・glyph・subroutine・charstring operation・outline segment・selected glyph・subset bytesの
上限は元CFF診断コードを保ったLimitとする。CFFのglyph closure／receipt不一致はInternal、
その他のCFF入力不正はInputに分類する。resourceのepoch・font instance・plan等の不一致は
I9190／Internalとする。型を直接分類するためCLIにローカルtypaxis-font依存を追加した。

実脚注font確定の記録数／spool必要量ちょうど・1不足でLimitとexit 5を確認し、別structureとの
不一致がInternalとなることも検証する。CFFの詳細なsource／phase情報の追加、公開writer／
manifest・CLIへの接続と元全巻・原ノ味等の受入ゲートは引き続き必要である。

ローカル検証: direct dependency追加後、`cargo test -p typaxis-cli --bin typaxis production_
-- --skip 5000`は164件成功・1件ignored（11.79秒）。
`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.70 ページ参照候補のgenerated text・shaping・行選択（実装追補）

`prepare_production_text_flow_with_page_references`は全Page-format参照について、source owner順の
明示した1始まりページ値を受け取る。sourceの参照集合を完全に覆うこと、ownerの厳密な昇順、
重複・未知owner・値0・max_pages超過がないことを検証し、記録数とgenerated text予算を確保前に
検査する。PageReference namespaceのgenerated buffer／provenanceを生成し、flowの検証も同じ
候補値から再構築して照合する。既存の引数なし経路は通常参照を未解決のまま保持する。

shapingとinline準備は、この明示したページ文字列を通常本文と同じ実フォント・文脈・cluster・
行選択へ渡す。shaping fingerprintではPageReferenceをFootnoteMarkerと別の種別として記録し、
既存脚注spanのエンコードを維持する。原文TextSpanや脚注markerへの偽装は行わない。

syntax testは完全被覆・重複・欠落・未知owner・値境界・候補改変を検証する。実CLI部品testは
候補1／12をshapingして2／3個の本文と参照のglyph clusterを確認し、行選択の強い検証と
flow fingerprintの変化を確認する。これらの候補値は実配置済みページの証明ではない。
実anchorからの値更新・ページと行の再帰的な収束・最終値の照合、Text／Number形式、公開
writer／manifest・CLI接続および全巻等の受入ゲートは引き続き必要である。

ローカル検証: syntax `production_flow_`は7件成功、ページ参照の実shaping／行選択testは1件成功。
`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`は165件成功・1件ignored
（13.58秒）。`--manifest-path workspace/Cargo.toml`と前節のtargetを使用した。

### 14.71 ページ参照候補の共通PDF接続と実ページ値の取得（実装追補）

共通本文・脚注driverに明示したページ参照候補を渡し、再組版・ページ配置・structure・font・
PDF assemblyまで同じgenerated textを保持する。脚注準備ではPageReferenceを脚注markerと
区別する。structureでは元sourceのReference nodeへ結び、generated key、buffer、各文字列と
全byte範囲の連続被覆を検証する。Labelや原文TextSpanへの置換は行わない。

sealed book selectionの`resolved_page_references`は、同じsourceのPage-format参照について
anchor名とownerを実destinationへ照合し、1始まりの実ページ値をallocationなしで取得する。
戻り順はsource traversal順であり、次候補として保持する側で予算計上とowner順への整列が必要。
候補値を実配置の証拠として扱わない。行・ページと参照値を再帰的に更新する収束処理、累積予算、
Text／Number形式、公開writer／manifest・CLIおよび元全巻等の受入ゲートは未完である。

実source-to-PDF testは先頭の空白ページ数0／1／11と候補1／12を組み合わせ、実ページ数と
参照先1／2／12、候補に一致するPDF ActualText、行とページの複数passを検証する。
ローカル検証: ページ参照testは1件成功（6組のPDF、0.42秒）。`cargo test
--manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ -- --skip 5000`
は165件成功・1件ignored（12.85秒）。targetは`/private/tmp/typaxis-vmb-book-build`、
最終ログは`/private/tmp/typaxis-page-reference-final-regression.log`。5,000画像と実全巻の
検証は今回のテストに含まれない。

### 14.72 実ページ参照の再生成と共通driverの収束（実装追補）

共通本文・脚注driverはPage-format参照を検出し、初期候補1から実destinationのページ値へ
更新してsource flow・shaping・行・ページ・PDFを再構築する。全参照のowner集合を保持し、
実ページ値と候補値が一致した完成結果について、display・book selection・PDFのfingerprintが
連続2回一致した場合だけconsumerを呼ぶ。誤った候補を描画できたことだけでは結果を返さない。
Text／Number形式や公開writerの認可へは拡張していない。

行探索とページ探索のworkは全再試行を通した残量を渡す。ページpassも同じmax_layout_passesから
減算し、内側のselect_stable_pages_with_pass_limitへ残量を渡す。このAPIは文書上限を増やせず、
残量2未満では配置開始前に拒否する。候補配列はsource順へ整列するための3配列分を確保前に
記録予算へ計上する。完成した各passのrecord／spool費用を累積し、上限超過でconsumerを呼ばない。
この完成pass費用の照合は全stageの一時allocationに対する事前課金の代替ではなく、その統合は
引き続き未完である。各passの借用graphは次の再試行前に破棄し、PDF全体のコピーは保持しない。

1／2／12ページへの参照で、正しい初期候補は2回、不一致の初期候補は更新と2回の一致確認を
合わせた3回で完了することを実PDFで検証した。共通driverの自動初期化でも同じPDFとなる。
探索work・完成pass records／spoolの必要量ちょうどと1不足、ページpass上限5／6を検証し、
不足時にconsumerを呼ばないことを確認した。関連CLI回帰は166件成功・1件ignored（12.61秒）、
ログは`/private/tmp/typaxis-page-reference-auto-regression.log`。コマンド・targetは前節と同じ。
桁数によって実改行・改ページが変わる書籍規模の検証、最終bidi、全allocation寿命管理、公開
writer／manifest・CLI、正式exporter、元全巻・原ノ味・規模・両hostゲートは引き続き必要である。

### 14.73 桁数による改行・参照先移動の収束検証（実装追補）

前節の収束処理について、候補値の桁数で実改行・改ページが変わる入力を追加した。
先頭10空白ページの後に「A 」とPage-referenceを置き、その次の段落を参照先にする。
本文幅1,600,000 raw、本文高さ1,100,000 rawでは1桁の候補は1行、2桁は2行となり、
候補1の参照先12ページが、候補12への更新後に13ページへ移る。候補13への更新と2回の
完成結果一致によって、計4回の再構築／8 page passesで確定する。上限7では結果を返さない。

このfixtureは数字の可視輪郭を持つ既存body-list-visible.ttfを使う。元のmetrics-only
フォントの空のASCII輪郭を描画成功の証拠にしない。検査用PDFの任意保存にはCLIの設定名と
衝突しないVMB_PAGE_REFERENCE_PDF_PROBEを使う。これは小規模なgenerated referenceの
描画／抽出／配置検証であり、日本語全巻・原ノ味・公開check/buildの受入証拠ではない。

ローカル検証: `cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis
production_`は5,000 distinct／alias画像を含む169件成功・1件ignored（260.02秒、
`/private/tmp/typaxis-page-reference-full-production.log`）。その後の可視フォントと上限7の
追加検証は`production_page_reference_reflow`で成功（0.57秒、
`/private/tmp/typaxis-page-reference-visible.log`）。targetは前節と同じ。

独立検査用PDFは`/private/tmp/typaxis-page-reference-reflow-visible.pdf`、SHA-256は
`dc6e2cf2d5d6415697babbfb6e1fce828ed5757cc9380f20d53df611095140e1`。
Poppler／MuPDF抽出で11ページ=A、12ページ=13、13ページ=Aを照合し、pdfinfo -destsで
targetが13ページのXYZ destinationであることを確認した。数字の可視輪郭はMuPDFとPopplerで
描画した。本文Aの輪郭はこの診断fontでは空であり、本文可視性の検証には含めない。
全プロセスは終了済みである。

### 14.74 最終PDFのページ参照closure（実装追補）

公開writer接続の前提として、PDF層にProductionPageReferencePdfClosureを追加した。
seal_production_page_reference_pdfは、実PDFとstructure／sourceの強い検証、同じnavigationに
結び付くbook selectionの検証を行い、全Page-referenceのgenerated値を実destinationへ照合する。
呼び出し側から期待ページ値や成功flagを受け取らない。flow・selected book・profile・limits・
PDFのfingerprint、参照数と記録費用をprivate fieldに保持し、verifyは同じ入力から再検証する。

closureは保持recordを1件追加する。本文・参照文字列やPDF全体を新たにコピーしない。
共通driverは、実ページ値と連続2回の完成結果が一致した後でこのclosureを生成・再検証し、
累積record費用へ加えてからconsumerを呼ぶ。closureはページ参照の正しさだけを証明し、
PDF/UA宣言、汎用組版の収束認可、VerifiedPdfBytesReceiptの発行を意味しない。

実PDFテストでは正しい1／12ページ値のclosure、誤った候補値の拒否、別PDFに対するclosureの
再利用拒否、record overflowを確認した。収束の必要record量ちょうど／1不足テストもこの1件を
含めて通過した。ローカルCLI check成功（55.50秒）、`cargo test --manifest-path
workspace/Cargo.toml -p typaxis-cli --bin typaxis production_ -- --skip 5000`は167件成功・
1件ignored（15.43秒）。targetは前節と同じ、ログは
`/private/tmp/typaxis-page-reference-closure-regression.log`。5,000画像の再実行は含まない。
公開writer／manifestの統合、全allocation予算、Text／Number参照、正式exporter、元全巻・
原ノ味・規模・両host受入は引き続き必要である。

### 14.75 共通PDFのsafe-vector closure接続（実装追補）

ProductionFootnotePdfAssembly::seal_safe_vectorは、完成した共通PDFのsource／structure／
object graph／bytesを検証した後、実際のvector contributionと絶対object/use観測値から既存の
StagingSafeVectorPdfClosureV2を生成する。公開serializer用APIは引き続きVerifiedPdfBytesReceiptを
要求し、raw bytes/hashによる入口はcrate内部だけとする。共通経路のために公開PDF認可を先に
捏造しない。共通driverはclosure生成をbook projectionの前へ接続し、その費用を引き継ぐ。

validation用のobject索引2組とpage索引・object集合をusage数による上界で記録予算へ事前計上する。
closure JSONは書き込み先を計数器にして長さを求め、spool上限確認後に必要量を確保する。
final-writer観測のverifyは同じcanonical serializerを既存文字列の比較sinkへ流し、観測JSON全体の
コピーを廃止する。既存algorithm／canonical JSON形式と公開serializerの検証条件は維持する。
全工程の一時allocation課金がこの局所接続だけで完了したとは扱わない。

実共通PDFでfinal hash・byte length・object count・writer fingerprintを照合し、record／spoolの
必要量ちょうど／1不足を検証する。CLI production回帰は167件成功・1件ignored（12.60秒、
`/private/tmp/typaxis-common-safe-vector-regression.log`）。5,000画像は今回再実行していない。
公開tagged/book closure・writer／manifest、正式exporter、元全巻・原ノ味・規模・両hostゲートは
引き続き必要である。

safe-vector単体テストは7件成功（0.15秒、
`/private/tmp/typaxis-safe-vector-closure-final-unit.log`）。既存の公開closureとの一致、closure JSONの
必要spool量ちょうど／1不足、観測JSONの末尾追加・欠落・先頭改変をhash更新後も拒否することを
確認した。全コマンドはworkspace manifestと前節のtargetを使用し、全プロセスは終了済み。

### 14.76 book観測JSONの中間コピー削減（実装追補）

book final-writer観測の生成・検証で、Info／language-paint／outlineのcanonical JSONを一度
Stringへ構築してからハッシュ化する処理をstreaming SHA-256 sinkへ置き換えた。生成時は同じ
serializerで長さを計数し、従来の保守的なspool課金を確認してからハッシュ化するため、既存の
境界値・canonical形式・fingerprintは維持する。再検証ではfinal-writer JSONも比較sinkへ流し、
丸ごとの再コピーを行わない。小さな数値・escape断片の一時文字列は残っており、全工程の
allocation管理が完了したとは扱わない。

日本語・絵文字・制御文字・引用符・backslashを含む値とSHA-256ブロック境界で、従来の
文字列serializerに対するhash／内容一致を確認する。余分な末尾、欠落、先頭改変は拒否する。
公開book validatorでも、改変したJSONに合わせてhashを更新した観測を拒否する。従来の
PDF/UAなしXMPの拒否は維持し、未完成の共通PDFに公開適合宣言を付ける変更は行っていない。

ローカル検証: PDF crateのbook_navigationテスト5件成功（0.25秒、
`/private/tmp/typaxis-book-stream-final-unit.log`）、CLI production回帰167件成功・1件ignored
（12.76秒、`/private/tmp/typaxis-book-stream-final-regression.log`）。workspace manifestと
`/private/tmp/typaxis-vmb-book-build`を使用し、5,000画像は今回再実行していない。
公開writer／manifest・book/tagged closure、全allocation管理、正式exporter、元全巻・原ノ味・
規模・両hostゲートは引き続き必要である。

### 14.77 脚注内ページ参照の収束検証（実装追補）

脚注本文がPage-referenceだけで構成される入力を共通driverへ接続して検証した。先頭11空白
ページを保持し、本文の脚注marker／定義labelは1、脚注中のページ参照は初期候補1から実値12へ
更新する。3回の再構築／6 page passesで収束し、source ownerがPageReferenceとして保持される。
脚注へのリンク注釈は1件だけで、ページ参照を追加の脚注markerや脚注配置要求として扱わない。

可視数字用の既存診断fontを使い、MuPDFで本文marker、separator、脚注labelと参照値12の描画を
確認した。Popplerは12ページに本文A1、脚注1／12を抽出し、MuPDFも同じ文字列を抽出する。
pdfinfo -destsはtargetを12ページへ解決する。検査用PDFは
`/private/tmp/typaxis-footnote-page-reference.pdf`、SHA-256は
`f648b5854739665379df8d20fc817714f65a79ec7630ae9124805b4eedd46893`。
診断fontの本文Aは空輪郭であり、日本語本文の可視性や全巻の受入証拠にはしない。

ローカルCLI production回帰は168件成功・1件ignored（12.57秒、
`/private/tmp/typaxis-footnote-reference-regression.log`）。workspace manifestと前節のtargetを
使用し、5,000画像は今回再実行していない。Text／Number形式、公開writer／manifest、全工程の
allocation管理、正式exporter、必要な戻りリンク、元全巻・原ノ味等の受入は引き続き未完である。


### 14.78 VMB文書JSONの確保前上限確認（実装追補）

VMB側EncodeBookBodyで、文書全体をmarshalする前に閉じたDTOのJSON byte数を計数する。
指定MaxDocumentBytesを超える入力は全体JSONを確保する前に拒否し、必要量ちょうどは従来と
同じJSONを生成する。UTF-8・HTML／制御文字escape・整数・数式DTOを含めて既存serializerと
一致を確認し、独自serializerは実行しない。interfaceを階層に数えず、wire深さ256を維持する。
詳細と検証はVMB正本§15.40を参照する。sidecar等を含む全allocation管理、正式exporterの
CLI／ArtifactSink接続、公開writer／manifestと全巻・原ノ味・規模・両host受入は未完である。


### 14.79 脚注定義番号と戻り先の結合（実装追補）

共通脚注navigationは、定義の最初の本文行を覆うforward destinationとは別に、生成番号だけの
logical boundsとLabel構造nodeを保持する。番号のclick領域を定義本文・数式・図まで広げない。
各定義の静的な戻り先はsource順で最初の参照とし、完成したreference link列のindexで保持する。
同一参照の番号が複数groupへ分かれた場合も先頭の実fragmentへ戻す。後続の参照から定義への
forward linkはそれぞれ保持する。viewerの履歴に応じて直前のclick位置へ戻る機能ではない。

参照探索用の定義数分の配列を確保前に既存record予算へ追加計上する。navigation fingerprintの
内部domainを/2へ変更する。CLI共通回帰では番号boundsを実drawから独立に集約して照合し、
複数参照でもsource順の先頭が選ばれることと、必要record量ちょうど／1不足を検証した。
最終コマンドはworkspace manifest・`/private/tmp/typaxis-vmb-book-build`を使う
`cargo test -p typaxis-cli --bin typaxis production_ -- --skip 5000`。168件成功・1件ignored
（14.91秒、`/private/tmp/typaxis-footnote-return-regression.log`）、全プロセス終了済み。

この変更は戻り先と番号領域の計画までで、戻りリンクのPDF annotation・対応するLink構造は
まだ生成しない。公開writer／manifestへの共通組版接続、正式exporter、全allocation予算、
元全巻・原ノ味・規模・両hostの受入とともに必要な残件として維持する。


### 14.80 共通PDFの脚注往復リンク（実装追補）

共通組版のregistry生成は専用入口build_structure_registry_v2_with_footnote_linksを使い、
source Note／Referenceと生成Lblの間に、それぞれ生成FootnoteLink（PDF role Link）を置く。
Linkは番号のaccessible nameと継承languageを持つ。従来のregistry生成APIの構造は変更しない。
再検証は保持された生成slotから同じ構造を再構築し、source／authorization／limitsとの一致を
確認する。追加node・深さ・文字列は既存registryの上限検査へ含める。

共通annotation生成は本文の参照番号から定義へのforward linkを新Linkへ所属させ、定義番号の
狭い領域から§14.79の最初の参照へ戻るannotationも生成する。通常のsource Linkの探索では
生成FootnoteLinkを専用navigationへ委ね、重複する通常リンクを作らない。ParentTreeの
StructParent、LinkのOBJR、子LblのMCRを既存の共通structure object生成・完成PDF検証へ通す。
戻り注釈もobject／record／spoolの計数と確保前上限確認へ含める。

19種類の共通fixtureで実annotationのRect／XYZ／Contents／構造nodeを検証し、複数参照では
forward linkを各参照に残して、定義番号から最初の参照へ戻すことを確認した。最終CLI production
回帰は168件成功・1件ignored（12.18秒、`/private/tmp/typaxis-footnote-backlinks-regression.log`）。
旧／新registryの個別再検証は1件成功（1.29秒、`/private/tmp/typaxis-footnote-link-registry.log`）、
layout-contractのtagged_structureテストは2件成功（0.06秒、
`/private/tmp/typaxis-footnote-link-contract.log`）。全プロセス終了済み。workspace manifestと
前節のtargetを使用し、5,000画像は再実行していない。

独立検査用PDF `/private/tmp/typaxis-footnote-backlinks.pdf` のSHA-256は
`ad1f09dddaa188cdb421091aba1263b725e6092b441d3c56961d7d43a2f94e0e`。
MuPDFは12ページの2 annotationと別々のLink／OBJR、子Lbl／MCR、およびParentTreeの対応を
読み出した。forwardのXYZは定義行、returnのXYZは本文参照番号の実位置を指す。Popplerは
本文A1と脚注1／ページ参照12を抽出し、MuPDF描画で番号・separatorを確認した。診断fontの
本文Aは空輪郭であり、日本語本文や全巻受入の証拠ではない。共通PDFには引き続きPDF/UAの
適合宣言を付けず、公開writer／manifest、正式exporter、全allocation予算、元全巻・原ノ味・
規模・両hostの受入は未完のままである。


§14.80の追加検証: 生成FootnoteLinkしかない場合は、通常navigationの文書全体の索引を
構築しない。脚注リンクの実処理は専用navigationで継続する。本文／脚注にauthored Linkが
ある場合は従来どおり索引を確保する。前者の通常navigation追加recordが0であることと、後者の
リンク・費用の保持を19-fixture試験へ追加した。回帰は168件成功・1件ignored（14.55秒、
`/private/tmp/typaxis-footnote-navigation-index-regression.log`）、全プロセス終了済み。
同じ診断入力のPDFは§14.80のPDFとcmpでbyte一致したため、同節の限定的な独立検査は維持する。
公開writer／manifest・正式exporter・全巻受入がこの最適化で完了したとは扱わない。

### 14.81 共通組版からのtagged PDF serializer（実装追補）

`write_production_common_tagged_pdf`は、共通組版のresource object graphから
PDFを生成し、package・profile・semantics・limitsへの構造registryの結び付け、
実際の構造／MCR／OBJR／ParentTree／metadata、Page参照値、safe-vectorとbookの
最終PDF照合を通した後、非Cloneの`ProductionCommonTaggedPdf`を返す。
この値は単一の`VerifiedPdfBytesReceipt`と実出力のobject観測・hashを所有する。
本文フォントは共通組版のfont planから使い、native mathの存在に依存しない。
既存の診断用assemblyはPDF/UA識別を付けないまま維持し、新しいserializerに限って
検証済みのgraphからPDF/UA-1識別を含むmetadataを生成する。callerがconformance
flagや最終writerの観測値を渡す入口は設けない。

19件の既存fixtureについて、veraPDF 1.30.2のPDF/UA-1機械検証が全件合格し、
stderrも空である。対象はTTF／TTC／CFF、本文・SVG数式・番号、PNG／JPEG、
通常リンク・目次、リスト、継続／反復脚注、language override等。
[機械検証のhash一覧](../samples/machine-package/staging/production-book-1/vmb-book/common-tagged-serializer-verapdf.json)と
[実際のveraPDF report](../samples/machine-package/staging/production-book-1/vmb-book/common-tagged-serializer-verapdf.xml)を保存した。
異なる書籍のprofile／semanticsを拒否し、追加record／spoolのexact／one-shortを
検証する。既存の診断PDFと新serializerでは、metadata以外のcontribution payloadが
byte単位で一致することを比較した。

独立検証で見つかったCID CFFのTop DICT順序も修正した。共通のsubset writerで
ROSを先頭operatorへ移動し、veraPDFの警告を解消した。原ノ味の5,052-byte選択subsetの
新SHA-256は`3b9ddbc2e0415a302c95f550113ddacfcc60d2f6fb219ad7f9296753f8823463`。
FontToolsの輪郭／mapping比較とFreeTypeの112描画比較は維持された。

この入口はまだ公開CLIの`build_production_book_pdf`やroot build manifestには
接続されていない。共通組版のtagged manifest観測、収束driverからの最終serializer呼出しと
累積予算引継ぎ、正式exporter、全allocation監査、元全巻・原ノ味全巻・規模・両host・
人によるPDF/UA確認等の受入は未完であり、19件の機械検証をその代替にはしない。


### 14.82 共通組版からのsafe-vector／math-vector manifest（実装追補）

`build_production_safe_vector_manifest`は共通page contentと最終tagged PDF、
book manifestを照合し、実drawの位置・意味上のfragment順序・数式terminalと、
実writerのPage／Content／Form番号から既存`typaxis.safe-vector-manifest/2`を生成する。
共通drawはsource bindingと選択された物理配置を一つのreceiptに結ぶため、
selected-placementとdisplay-commandの両fingerprintはその実drawを参照する。
frame indexには共通組版の物理fragment indexを用いる。Formを共有するaliasごとの
使用fingerprintを保持し、未使用resourceのForm番号はnullのままにする。
content・image・object role・math flowの索引を用い、aliasごとに全配置を再走査しない。

`build_production_math_vector_manifest`はそのsafe manifestと同じ共通display／limitsを
要求し、実math bindingのTeX source・alternative・metrics・provenance・式番号情報を、
対応する最終safe-vector usage fingerprintへ結び付ける。元のbinding-set fingerprintを
保持し、既存`typaxis.math-vector-manifest/1`のfact encoderを共用する。
owner索引で対応を確認し、重複owner、欠落した数式usage、異なるdisplay／limitsは拒否する。

両入口は先行phaseのrecord／spool chargeを引き継ぎ、索引・保持record・source文字列と
canonical化の上限を事前加算する。各入口について最終PDF生成までを含むexact／one-short
予算試験を追加した。19件の共通fixtureでは最終PDF hash・object番号・viewport／matrix・
alias使用数・数式source／binding／usageを確認し、異なる書籍profileとlimitsを拒否する。
既存manifest encoderの回帰も確認した。これは全pipelineのallocation監査や規模受入の
完了を意味しない。tagged manifest観測、公開CLI・root manifestへの接続、収束driverの
累積予算引継ぎ、正式exporterおよび全巻等の受入は引き続き未完である。


### 14.83 共通PDFのtagged観測とmanifest（実装追補）

共通serializerは実graphの検証後に`ProductionCommonTaggedObservation`を発行する。
`typaxis.production-common-marked-content/1`は、全ページの実marked streamの
byte length／hashと、実drawを構造・MCIDへ結び付ける共通structure fingerprintを記録する。
`typaxis.production-common-tagged-pdf-observation/1`は、それに加えて全出力objectの
番号／offset／byte length／hash、最終PDF hash、structure registry、およびvectorごとの
実MCID・Page／Content／StructElem番号・paint／semantic fragment順序を記録する。
共通structure receiptをselected-binding identityとして使い、旧staging選択のreceiptを
合成しない。観測値はprivate factoryが同じ検証済みassemblyから取得し、外部から
object番号や適合flagを注入できる入口は設けない。

`build_production_tagged_manifest`は、この観測と共通structure、safe／math manifestの
identity・source kind・metrics・language・usageの対応を確認し、既存
`typaxis.tagged-pdf-manifest/2`を生成する。既存builderとcanonical encoderを共用し、
math sourceだけにmath binding参照を残す。異なる最終PDF／limitsは拒否する。
観測とmanifestの追加record／canonical bytesを確保前に加算し、先行phaseからの
累積record／spool上限を引き継ぐ。serializerの既存予算試験と、新tagged manifestの
exact／one-short試験でこの引継ぎを確認する。

19件の共通fixtureで実MCID・object番号・object payload hash・marked stream hash・
canonical順序とmanifest参照を照合した。book／safe／math／taggedの4 memberが既存の
root dependency検証を通ることも確認した。再生成した19 PDFのhash／sizeは§14.81の
veraPDF合格証拠と全件一致した。PDF library 85件（1件ignored）、manifest library 38件、
CLI production回帰173件（1件ignored）が成功した。これは公開CLIを切り替えた証拠ではない。
収束driverからの最終serializer呼出し・累積予算引継ぎ、root側の保持／複製費用の接続、
正式exporter、全allocation監査および全巻・原ノ味・規模・両host等の受入は未完である。


### 14.84 収束driverから最終serializerへの予算引継ぎ（実装追補）

`with_production_common_tagged_pdf`は既存の共通本文・脚注／Page参照収束driverを
通した後、同じ診断assemblyのsource graphで最終tagged serializerを呼び出す。
`write_production_common_tagged_pdf_after_assembly`は先行assemblyを再検証し、
収束ownerから受け取った累積record／spoolを最終組み立ての開始値にする。
source graphの費用へ戻して過去passを払い戻すことも、同じsource graphの費用を
もう一度加算することもない。先行assembly自身の費用を下回る開始値は拒否する。
累積値の集計はCLI収束ownerの責務であり、serializerが未知の過去passを推測する
仕組みではない。最終closureの追加費用はその開始値から通常どおり加算する。

callbackには所有された最終PDFと、同じ診断assemblyの読み取り専用source graphを渡す。
これにより共通book／safe／math／tagged manifestを同じ選択から作成できる。
複数passのPage参照fixtureで、直接serializerとのPDF byte一致と、両費用の差が
「収束累積値 − source graph費用」に等しいことを検証した。参照あり／なしの入力で
収束から全4 manifestまでのexact／one-short record／spool試験を追加した。
CLI production回帰175件（1件ignored）、PDF library 85件（1件ignored）が成功した。

公開`build_production_book_pdf`はまだ旧writerを使う。新driverからroot manifestとtraceを
保持する費用の接続、および公開入口の切替を行ってから、正式exporter・全巻等の受入を
検証する必要がある。この引継ぎの試験だけを公開build成功の証拠にはしない。

### 14.85 共通traceのnative数式と表計測の識別（実装追補）

共通公開writerのtraceは、従来の`block_layout_sha256`が指すprecomposed block
receiptに加え、`native_math_layout_sha256`と`table_measurements_sha256`を出力する。
前者は最終行配置が借用するnative computation owner、後者はmixed page searchが
使用した同一の実測cell collectionのfingerprintである。該当するownerがない場合は
`null`とし、空の架空receiptやsource hashで代用しない。mixed sequenceの発行元検証でも
実測fingerprintを照合する。これは診断用の識別情報であり、hash単体を新しい配置権限にしない。

両fieldは対で共通traceにだけ許可し、hashの形式とcontract 1.4をschemaで検査する。
従来のtraceを保存済み証拠として読めるよう、両fieldがない記録の受理は維持する。
raw package、capabilities、root manifestの公開fieldは変更しない。固定サイズの追加値は
既存root traceの事前確保枠内に収め、native receiptや表計測をtraceのために再計算しない。
PDF、selected layout、描画・構造の各receiptによる最終closureは引き続き別途必要である。

### 14.86 アウトラインの実構造参照と独立PDF検証（実装追補）

共通object projectionはoutline source ownerを同じStructureRegistryのsource nodeへ
解決し、各outline itemの`/SE`を実StructElem objectへ結ぶ。registry indexの容量を先に
課金し、一度の整列とsource owner検索で解決する。resource contributionが持つ構造参照は、
借用中のstructure ownerで確認したうえで、最終object番号へ解決する。見出しroleの一致だけや
source node IDをPDF object番号へ変換する推測で代用しない。

独立した複合corpusのPDF検証では、脚注参照と戻りラベルのLink、Lbl、OBJR、ParentTree、
実ページ、注釈ContentsとMCID所有のActualTextを結び付ける。戻り先は参照ラベル矩形の
左上へ一致させる。順方向は対応Noteのページとラベルを含む座標関係を検査するが、検証器だけで
最初の内容行の全logical geometryを再組版したとは扱わない。ActualTextの総集合を保った
ラベル交換でも失敗させる。Page参照の文字列は実destination pageから導出し、旧writerの
`page target`というplaceholderを期待値にしない。outlineの`/SE`はsource traversalから
導出した正しいStructElemへ一致させる。外部抽出・描画・PDF/UA検証と全巻受入は別ゲートである。


### 14.87 公開manifestと共通traceの実反復回数（実装追補）

共通公開buildは、最終callbackの`observation.page_passes`をroot manifestの
`layout.pass_count`へ渡す。Page参照がある場合は、参照値を更新した各roundの実page
selection回数を含む累積値であり、最終roundだけの回数や行のreshape回数ではない。
現在の共通経路は収束時だけ公開するため、`selected_state`も同じ最終回数になる。
固定値1の投影を廃止し、2以上かつ同じeffective `max_layout_passes`以下を公開前に検査する。
累積値の集計責任は既存のCLI収束ownerにあり、このscalarを新たなPDF描画権限や
過去passの再検証receiptにしない。既存の最終PDF／選択fingerprint照合を維持する。

共通traceにも`pass_count`と`selected_state`を対で出力し、独立manifest検証で
root値・収束status・最終選択との一致を検査する。schemaは共通contract 1.4の範囲、
非零u16と現在の最小2回を検査する。保存済み旧traceは両fieldがないまま読めるが、
その反復回数を推測して補わない。追加fieldは既存固定サイズtrace事前確保枠内に収める。
これは実回数のsummaryであり、全候補や各passの状態を保持する汎用trace統合の完了ではない。

### 14.88 共通shaping失敗の型付きsource診断（実装追補）

公開buildの組版失敗を`Failure.message`のDebug表現だけに変換すると、canonical診断の
input-snippet拒否によりmessageが一般文へ置き換わり、locationもpackage rootへ落ちていた。
実VMBの和文font不足で、private errorにnode 3があるのに公開`L5100`はroot位置だったことを
再現した。原著位置をVMB側で推測する修正は行わない。

共通body reshapeの`Shape`と`Layout`は、型付きerrorのownerから既存1.4のsource locationを
組み立てて`Failure`へ保持する。`MissingShapedGlyph`は実TextSpanも保持し、それ以外に
未知のsource byte rangeを補わない。shapingは閉じたreasonとphaseをcanonical noteに残す。
inline準備はそのphaseとownerを保持する。公開emitはこの診断を使い、従来のexit分類・private
cause・失敗manifestを維持する。source ownerを持たないfeedback errorの位置は創作しない。

VMBは同じStageBookPackageで確定した原著provenanceへ逆引きし、生成projectionのoffsetと
原著JSON/TeXのoffsetを区別する。実CLIのfont不足は`node_id=3`、
`phase=authored-text-shaping; reason=missing_declared_font_coverage`を返し、VMBが
`book/chapter.json`の`/blocks/0`へ対応付けた。VMB側の対応契約・上限・実行証跡は
VMB設計§15.47と実装台帳を参照する。これは全pipelineの位置付き診断や正式renderer公開の
完了ではなく、型付き情報を保持できる共通shaping／inline準備経路の接続である。

### 14.89 VMB通常図版と共通PDFの接続（実装追補）

VMBの明示選択されたblock figureは、数式SVGと同じdense image resource列へ接続する。
通常画像はmath IDの後ろに配置し、同じ元bytesを共有する各配置のsource origin・Alt・caption・
番号presentation・幅とkeep設定を保持する。captionの数式も既存の実math loweringを通る。
JPEGは既存profileの先頭JFIF APP0要件を維持し、非JFIF baselineを暗黙変換しない。
詳細なVMB契約はVMB設計§15.49、実行結果は実装台帳の対応節を参照する。

小規模な和文混在例でPNG共有2配置とJPEG1配置、実SVG2 resource/5 useの公開buildと独立PDF
検査が成功した。元画素・alpha・全SVG path・画像幅/縦横比・source順のraw文字抽出を照合し、
PDF/UA-1機械検査106 rules / 1,118 checksに失敗はない。この証跡を全巻51図版の受入や
人による出版承認へ拡張しない。caption/prefixなしのfigureには現契約でanchorを置かず、
その参照は未解決として拒否する。正式renderer公開と残る全巻入力・組版条件は未完である。

### 14.90 VMB用語集の本文・参照先への接続（実装追補）

VMBの明示設定された用語集は、実preferred termとrich definitionを共通本文へ投影し、
定義の数式も既存source math ownerを使う。concept IDは実anchorとなり、本文リンクと
用語集outlineを共通PDFへ接続する。表示外の関連概念・別名・SymbolTeX等は元JSONのまま
source sidecarに保持する。LocationがないRenderBook entryから原著位置を推測せず、
callerが供給したoriginを検査・copyする。VMB設計§15.50に境界と保持上限を定める。

小規模例は内部リンク1件・outline2件・実数式3配置を独立照合し、PDF/UA-1機械検査
106 rules / 778 checksに失敗0。全巻418用語の受入、参考文献・verification companion、
正式renderer公開、runtime独立検査と原ノ味等の全巻条件をこの結果で完了扱いにしない。

### 14.91 VMB参考文献・引用の元番号と実リンク先（実装追補）

VMBの明示bibliography表示は元entry順のordered listを生成し、prepared citation番号と
同じ1始まりinventoryへ結び付ける。Prefix/Suffix/Locator・実数式を保持し、文献IDの実anchorへ
リンクする。完全な書誌情報と引用payloadはsource sidecarに残す。表示policy・上限・originと
改変拒否はVMB設計§15.51を参照する。生成UIラベルは出版言語を継承し、引用先文献の言語を
ラベルへ誤って流用しない。既存Typaxisの閉じたprofile条件を緩和しない。

独立PDF検査は、内部Destと外部URI actionを区別し、元target byteとaccessible nameへ照合する。
追加action、URI/内部Dest併用、別URLへの変更を拒否する。小規模2文献・2引用例では5 linksと
言語別textの保持を確認し、PDF/UA-1機械検査106 rules / 1,961 checksに失敗0。
これは全巻の全引用・全参考文献やverification companion、正式renderer公開・原ノ味等の
残る受入条件の完了ではない。

### 14.92 verification QR生成物のresource境界（実装追補）

VMBはverification URLからbounded QR artworkを生成し、元URL・producer・module/quiet-zone・
pixel倍率・PNG hashを結ぶ。PNGの完全admissionと物理配置は既存Typaxisの責務を維持する。
通常画像や数式をQRへ置換せず、QRをFormulaとして扱わない。VMB設計§15.52に生成契約と
上限を定める。章/結果への正式loweringとHTML・検証metadataの接続はまだ未完である。

小規模Figure例では全32,400画素と18mm相当の幅を元resourceへ照合し、元PNGとPDF全pageの
300dpi描画の両方から別decoderで元URLを読めた。PDF/UA-1機械検査106 rules / 491 checksに
失敗0。この証跡はQR生成resourceの受入であり、形式証明の再検証・companion公開・全巻の
受入を意味しない。

### 14.93 VMB章companionの見出し・QR・HTMLの一貫性（実装追補）

VMBの明示chapter companionは実chapter/appendix見出し直後に生成QR figureと元URLのlinkを
配置する。全prepared result metadataを元配列index付きJSONで保持し、元PathへCSP付きXHTMLを
保存する。章Locationと生成物originを区別し、図版・数式は共通resource/PDF経路を使う。
重複・孤立章・不正path/URL・未知status・改変は拒否する。VMB設計§15.53に上限と所有契約を定める。
保存済みHTMLの変更/削除/追加もcommand起動前に拒否し、実行中はinput snapshotで検査する。

小規模例のPDFは54,740 bytes / 1 page / 48 objects / 2 passes。独立検査で見出し直後の配置、
元link/HTML result情報、全32,400 QR画素・実数式2 Forms/2 usesを確認し、PDF全pageの300dpi
描画から元URLを復号できた。PDF/UA-1機械検査106 rules / 491 checksに失敗0。
これは章companionの接続であり、結果blockの検証表示/formal metadata、正式renderer公開と
runtime独立検査/ArtifactSink、元全巻/原ノ味・人の受入は引き続き未完である。

### 14.94 VMB結果検証表示の本文とsource metadata（実装追補）

VMBの明示verification設定は結果・演習のFormalResultIDと全VerificationDisplayを保持し、
元badge/短縮ID/証明書hash/kernel情報を通常本文へ接続する。LinkStyleの5形式をprofileと照合し、
直接linkと結果QR、章companionの対応を検査する。章形式は形式結果ID・status・短縮IDを照合し、
全一覧結果が実表示へ対応することを要求する。元データと生成物のorigin・順序・wireを束縛し、
改変を拒否する。VMB設計§15.54に所有契約と上限を定める。

実direct・結果QR・章QRの3形式で公開buildと独立PDF検査が成功し、元display情報・link・
数式2 Forms/2 uses・各QR全画素と文字順を照合した。PDF/UA-1機械検査は各106 rules、
1,269/1,293/1,307 checks成功、失敗0。QRは元PNGとPDF全pageの300dpi描画から別decoderで
元URLを得た。FormalStatement/NPA source、正式renderer/runtime独立検査/ArtifactSinkと全巻条件は未完である。

### 14.95 VMB FormalStatement・NPA原本と本文の対応（実装追補）

VMB側の明示formal設定を結果・演習へ接続した。inline/collapsibleは対応結果の後へ静的表示し、
appendixは入れ子を含むassembly順で巻末へ集約して本文から実anchorへリンクする。
formal名、result/statement unit IDs、mode、元resource情報をsource sidecarへ保持する。
manifestで宣言されたformalSourceだけをsize/hash/UTF-8検査して読み、元bytesを排他的にstagingする。
共有sourceでも各出現の元pathと行byte spanを保持し、起動前にsourceファイルの変更・欠落・追加を拒否する。

NPAは通常の選択済み本文fontのliteral textで表示する。CRLF/CRは明示break、tabはcaller指定の
論理columnへ展開し、元ファイルの改行・tab・終端LFは無変更で保持する。HTML/TeXを評価しない。
未対応制御文字の診断は元path/line/byte spanへ戻る。元formal/verification metadataの保持であり、
証明やcertificateの正当性を新たに認証する処理ではない。

実3モードの公開PDFは1 page/2 passes。inline/collapsibleは86,633 bytes・66 objects、appendixは
91,672 bytes・73 objects。元NPA153 bytes、formal JSON、行span、本文順、リンク先と全2 math Formsを
独立検査し、PDF/UA-1は各106 rules、1,566/1,566/1,707 checks成功、失敗0。
Go全体回帰132 tests成功、2 skip、184.540秒。再生成PDF/原本は独立検査対象とbyte一致した。
[検証台帳](../samples/machine-package/staging/production-book-1/vmb-book/vmb-formal-external.json)を参照する。
正式renderer/runtime独立検査/ArtifactSink、残るRenderBlockと元全巻/原ノ味等の受入条件は未完である。

### 14.96 次期契約の型付き囲み・解答carrier（実装追補）

[ADR-0039](../adr/ADR-0039-book-2-semantic-vocabulary.md)は、元全巻の260解答と328囲みに必要な
次期semantic kindを固定した。1.5では従来のresult/proof/exerciseに加え、solution、example、
counterexample、remark、note、warning、common_error、formalization_note、quoteを型で保持する。
これらを1.4の3種類へ読み替えない。description listを含む残る本文要素も全巻の必須条件である。

`typaxis-document-package`の再帰wire treeを閉じたkind型で共有し、1.4の既存API名は元の3種類を
指定した具体的な型aliasとして維持した。共有decoderはその型のcontractだけを受け取り、canonical
JSONと元bytesのhashを別々に保持する。旧serdeの型名とdecoded debug表示も維持する。
1.5用`book_v2` moduleはtestまたは`book-v2-staging` featureだけに存在し、normal CLI/current
contract/profile/capabilities/schema aliasへ登録しない。source/syntax/profile/layout/PDFの受入receiptを
このcarrierから捏造せず、次の共通組版接続へ渡す非公開入力として扱う。

検証は、新12種類×一般8配置先のtyped/canonical往復と、native math/vector/JPEG/CFF/navigationの
既存6 carrierの不変性を含む。文書packageは51 unit・4 property・1 compile-fail成功、既存syntaxは
72 unit・6 compile-fail成功、共通container配置3件とnested配置1件も成功した。
通常公開CLIを再buildし、capabilitiesのbyte一致、旧1.4 schema全29件のbyte一致を確認した。
実公開checkは1.5をP1103、1.4へ混ぜたsolutionをP1102としてresource欠落診断より前に拒否する。
VMBの旧1.4 formal 3モードは再buildで前回の独立検査済みPDFと全bytes一致した。
[証跡](../samples/machine-package/staging/production-book-1/vmb-book/book-v2-carrier-external.json)に
コマンドlog・コード・診断・capabilities・PDFのhashを保持する。新kindのPDF成功を意味しない。

### 14.97 次期本文の型付き準備と出典検証（実装追補）

`typaxis-document`と`typaxis-syntax`に`book-v2-staging`限定の本文準備処理を追加した。
再帰domainはkindを型引数として共有し、既存`StagingM4*`型名は1.4の閉じた3種類のaliasを維持する。
1.5の12種類は別enumと`BookV2*`型で保持し、旧kindへ変換しない。共有loweringはdense preorder、
親子spanの包含と順序、classの正規順序、UTF-8境界、native mathとprecomposed vectorの出典mapping、
本文共通のnode/depth/text budgetを検証する。全12種類×8再帰配置先は実際にdenseなIDを用いた
syntax試験を通過し、kind、node ID、source spanと元のwire document全体が保持される。

`PreparedBookV2Body`は元wire、型付き本文、resource宣言、unstyled native math、vectorごとの座標・
出典TeX・代替文・式番号、raw/canonical hash、同一limitsを所有する。入力の後続変更はこの結果を
変更しない。vector検証では1.4のsession・metrics receiptを発行せず、未発行の検証事実を保持する。
1.5の本文またはvectorを1.4の検証済み型へ渡す操作はcompile-fail試験で拒否される。

この段階は**本文準備**であり、styleの確定、host原本の受入、resource bytesの受入、次期profile、
flow/layout/structure/PDFの受入ではない。公開current contract、profile、CLIへの1.5登録は行わない。
description list、残るVMB本文変換、Harano実フォント、全巻と管理host等の必須ゲートも残る。

本文準備のコードは`cc99bc1`。feature有効のdomain 2 unit、package 51 unit・4 property・1 compile-fail、
syntax 79 unit・8 compile-failが成功した。通常CLIの囲み配置3件・nested配置1件も成功した。
再buildした公開CLIでcapabilitiesのbyte一致、凍結1.4 schema 29件のhash一致と1.5/P1103拒否を
再確認した。新kindの試験はフォント数上限で先に拒否されていたため、種別拒否の証拠には数えない（§14.100参照）。VMBのformal 3モードは14.508秒で成功し、inline/collapsibleの
86,633-byte PDF、appendixの91,672-byte PDFは前回の独立検査済みPDFと全bytes一致した。
新たなPDF/UA検査を実施したという主張ではない。コマンド、コード・実行ファイル・入力・PDFのhashと
[本文準備の証跡](../samples/machine-package/staging/production-book-1/vmb-book/book-v2-body-external.json)を保持する。

### 14.98 次期の囲み・数式スタイル確定（実装追補）

`typaxis-style`にも`book-v2-staging`限定の12種類のstyle kindとcomputed styleを追加した。
既存の1.4 style kindは3種類のままであり、共通cascadeの結果は入力したkindの型を保持する。
syntaxの再帰style収集はこの型を維持し、本文・囲み・list・table・caption・footnoteの全96配置例で
元のkindとnode IDに対応するcomputed container styleを確定する。`important`、specificity、
`extends`、font family/size/line-heightとtext-alignの継承は既存の閉じたregistryと計算処理を共有する。

`StyledBookV2Body`は本文準備結果と囲み・native math・vector blockのcomputed styleを所有する。
native mathは既存のparsed nodeを参照してスタイルを確定し、元TeXや解析ASTを複製・再解析しない。
vectorの式番号fontも実際の親から継承する。metricやsource TeXをスタイルで変更しない。
1.5のcontainer styleを1.4のstyle型へ渡す操作はcompile-fail試験で拒否される。
不正selector/property/extendsは読込境界およびstyle処理自体で拒否する。

feature有効のstyle 14 unit・1 compile-fail、syntax 82 unit・8 compile-failが成功した。
これは囲み・数式のスタイル確定であり、次期semantic/profile/host/flow/layout/structure/PDFの
受入をまだ構成しない。通常段落のtext flow、原本・resource admissionと全巻等の必須ゲートは残る。

スタイル接続コードは`3f122b9`。通常CLIの囲み3件・nested 1件も成功し、再buildしたCLIの
capabilities、旧1.4 schema 29件、1.5/P1103拒否は不変だった。
新kindの試験はフォント数上限で先に拒否されていたため、種別拒否の証拠には数えない（§14.100参照）。
VMB formalの3モードは14.829秒で成功し、PDFは3件とも前回の独立検査済みPDFと全bytes一致した。
[スタイル接続の証跡](../samples/machine-package/staging/production-book-1/vmb-book/book-v2-style-external.json)に
コード版、コマンド、35ファイルのhashと各runに保持した実行ファイル・PDFを記録した。

### 14.99 次期本文の言語・参照先索引（実装追補）

`book-v2-staging`限定で`PreparedBookV2Navigation`を追加した。元の`StyledBookV2Body`を借用し、
同じhashの再parse結果へ差し替える操作も拒否する。1.4と共有する再帰collectorは閉じたkind型を
保ったまま、本文・囲み・list・table・caption・footnoteとheader/footerの言語所有者を収集する。
元の12種別×8配置先で本文・style・言語索引のnode IDとkindの対応を確認した。

明示BCP47表記と実際の親からの継承を確定し、vectorのノード・種別・source span・明示言語を
元の準備結果へ照合する。式番号は親の言語を共有する別childであり、独立ownerにしない。
本文で計上済みのvector言語を二重計上せず、metadata、outline label、他の言語所有者を同じ
text budgetへ加える。wire側のmetadata/outline/page-region nodesとnative math ASTも同じnode
budgetで照合する。ヘッダーの宣言source範囲外spanは、1.5限定のtyped理由と該当spanのJSON Pointerを
持つP1102として返し、内部整合性エラーにはしない。

metadataとoutlineの検証事実を既存receipt生成から分離し、次期索引から1.4 navigation receiptを
発行しない。outlineの元kind・anchor・span・見出しlevel・親子関係・言語を保持する。内部リンクと
Text/Page/Number参照、脚注参照は全再帰本文から収集し、実在targetを検証する。visible labelや
配置pageをここで生成・推測しない。これは本文フローの前提となる索引であり、次期profile、host原本・
resource admission、layout/structure/PDFや全巻成功の証明ではない。

syntax 92 unit・9 compile-failが成功した。package側の共有再帰処理は51 unit・4 property・
1 compile-failが成功した。試験は全12 kindのoutline、前方参照・脚注・内部リンク、不正target、
重複anchor、metadata時刻/keywords、見出しsourceとoutline親、header言語、元bodyの所有権、
vectorの事実との照合と二重計上防止、text/ASTの上限ちょうど・1超過を含む。

索引コードは`f14b73471f2d3ddefd42705fcad04cdea89ac197`。通常CLIのnavigation golden 1件と、
明示的に起動した独立pypdf 6.10.0検査1件が成功した。後者は既存2-page PDFの14 objects、
3 outlines、1 link、metadataと言語、およびpath alias間のPDF/manifest byte一致を確認する。
公開CLIのcapabilities、旧1.4 schema 29件、1.5/P1103拒否は不変だった。
新kindの試験はフォント数上限で先に拒否されていたため、種別拒否の証拠には数えない（§14.100参照）。
VMB formalの3モードは15.852秒で成功し、PDFは3件とも既存の独立検査済み出力と全bytes一致した。
[索引の証跡](../samples/machine-package/staging/production-book-1/vmb-book/book-v2-navigation-external.json)に
33ファイルのhashとコマンドを記録した。新1.5 PDFまたは新たなPDF/UA検査の成功を意味しない。

### 14.100 次期本文フローの共通収集処理への接続（実装追補）

`PreparedBookV2TextFlow`を`book-v2-staging`限定で追加した。既存の本文フローstorageと再帰collectorを
共有し、source側の関連型で1.4の3 kindと1.5の12 kindを分離する。専用constructorは元の
`StyledBookV2Body`と`PreparedBookV2Navigation`を借用し、同じhashの再parseや、同じ本文から別途
作った索引への差し替えを拒否する。次期フローを1.4用フローとして渡す操作はcompile-failとなる。

通常本文は元text bufferを借用し、source span、実際の親から継承したfont・行高・配置、言語と
Text/Page/Number参照を保持する。囲み、list、table、caption、footnoteの境界をsource順のeventに残す。
表のhead/body、row/cell、rowspan/colspanと列幅指定は共通のtyped topologyとして保持し、占有gridと
保持recordを走査・確保の前に計上する。脚注本文を通常本文へ平坦化しない。list markerと脚注番号、
候補page labelはgenerated namespaceに置き、索引までの保持文字量へ合算する。候補page番号は全対象、
順序、重複、ページ数上限を検証するが、最終配置の証明にはしない。Text/Numberのvisible labelはまだ作らない。

新フローは`typaxis.book-2-source-text-flow/1`の識別子と、この段階が使うlimitsの明示的な射影を使う。
1.4の`typaxis.production-text-flow/7`とそのencodingは変更しない。`verify_for`は元ownerの一致を確認し、
全event、paragraph/style、table、list、figure、footnote、生成文字とfingerprintを再計算して照合する。
この成果はsource flowであり、host原本・resource bytes・font・次期profile・selected layout・structure・
PDFの受入ではない。description list、残るVMB本文変換、全巻と原ノ味・管理host等のゲートは残る。

コードは`f23251647bfb194f8739b3fd6fc25a5fdbcdb8f9`。syntax 100 unit・10 compile-failが成功した。
12 kind×8配置先のフロー対応、通常本文の既存経路との一致、source textの借用、候補page、原本owner、
継承style、改変検出、本文文字量と表recordの上限ちょうど・1超過を含む。CLI囲み配置3件も成功した。
公開CLIを再buildし、capabilitiesと旧1.4 schema 29件の不変を確認した。VMB formal 3モードは15.38秒
（Go package全体28.508秒）で成功し、PDFは3件とも以前の独立検査済み出力と全bytes一致した。
[本文フローの証跡](../samples/machine-package/staging/production-book-1/vmb-book/book-v2-flow-external.json)に
41ファイルのhash、診断と実行コマンドを保持する。新たなPDF/UA検査を実行したという主張ではない。

**公開拒否試験の訂正:** §14.97〜14.99の新kind拒否試験は、実際には2 font宣言が1 font上限を超える
P1102で先に停止していた。その3回をkind検証成功に数えないよう台帳を訂正し、元の診断は保持した。
今回の最終試験は未使用の第2font宣言を除き、毎回新しい診断出力先を使った。1.5/P1103、
1.4内の`solution`/P1102を確認し、stderrの理由も`unknown semantic_container semantic_kind`と照合した。
同じ入力の`solution`だけを`result`に戻す対照試験は原本読込まで進み、意図的に原本を置かないため
I9112を返す。診断sidecarが既存の場合は再公開が拒否され、古いJSONは残るため、終了codeだけで
新しい診断が保存されたとは扱わない。

### 14.101 次期本文の原本受入と実出典mapping（実装追補）

`typaxis-machine-input`の`book-v2-staging` featureに、次期PACKAGEのstable readからdecode、原本読込、
受入完了までを管理する`HostBookV2InputSession`を追加した。安全なPACKAGE読込は既存transportを共有するが、
1.5のdecoded/progress/source-set/inputは専用型を使い、1.4のcontract enumや受入receiptを発行しない。
`typaxis.book-2-host-input/1` fingerprintは元PACKAGE bytes・canonical hash・portable URIと、実際に
読み込んだ原本のID・bytes・hash・portable URIを結ぶ。絶対rootとsession IDをportable hashへ入れない。
同じbytesの別sessionのreceipt交換、異なるlimits、段階の巻き戻し・再decodeは拒否する。

原本宣言は非空かつdense ID順とし、個数を`max_include_files`、個々の長さを`max_source_bytes`、
原本の論理byte合計を`max_input_bytes`で制限する。PACKAGE自体は別のpackage byte上限で計上する。
宣言合計を全件preflightしてから、全原本候補を共通read ledgerへ登録し、最初の原本を開く。
途中で原本が欠落しても、後続原本はsidecar上書き保護のread setに残る。登録済み候補を使うため、
候補の作業量を読込時に二重計上しない。各原本は同一handleのstable bytesとして長さ・hash・UTF-8を検証する。
既存1.4の単一原本読込も同じ下位helperを使い、元のprogressとfingerprint発行を維持する。

syntaxの`prepare_admitted_book_v2_body`はこの専用受入結果を消費し、実原本を保持する
`SourceAdmittedBookV2Body`を作る。source bufferはコピーせず所有権を移す。TextMapはtext bufferの
全体を隙間・重複・空segmentなしに覆い、双方のUTF-8境界で切れることを確認する。identity segmentは
実原本sliceとのbyte一致を必須にし、replacementとinsertedは元の種別・出典有無を保持する。
本文全node、list/table/footnote/caption、式番号、header/footer textのspanも実原本のUTF-8境界へ照合する。
失敗時にはtyped理由と、受入済み段階・共通read ledgerを保持する。resource bytes、次期profile、
selected layout、structure/PDF、全巻の受入はこの型から発行しない。

コードは`d9021ddd0844be83e692fd3ee86bac560f6b2503`。machine-inputは22 unit・6 compile-fail、
syntaxは106 unit・11 compile-failが成功した。複数原本、別rootでのfingerprint一致、stable read後の
host変更、buffer所有権の維持、別session・limits・段階の拒否、個数・合計量・不正ID・URI・欠落・長さ・hash・
UTF-8、全候補の事前登録、実原本と異なるidentity文字列、mappingの穴・空区間・UTF-8境界、明示replacement/
inserted、原本node span、本文フローまでの接続を含む。

通常CLIを再buildし、capabilitiesと凍結1.4 schema 29件のbyte/hash一致を確認した。新しい診断出力先で
1.5/P1103、旧契約内`solution`/P1102とstderr理由を確認し、`result`だけに戻す対照は原本読込へ到達した。
VMB formalの3モードは14.19秒（Go package全体14.659秒）で成功し、PDFは3件とも以前の独立検査済み出力と
全bytes一致した。[原本受入の証跡](../samples/machine-package/staging/production-book-1/vmb-book/book-v2-host-source-external.json)に
39ファイルのhashとコマンドを保持する。新しいPDF/UA検査または次期全巻の成功を意味しない。

### 14.102 既存production入力の原本対応検証

`parse_admitted`でもbook-2と同じ原本検証を適用した。安定して読み込んだsourceとidentity text bytesの一致、全bufferを覆う非空mapping、source/textのUTF-8境界を検証し、違反時は資源admission前に既存`P1102`で停止する。これは[既存source規則](03-source-text-and-parser.md)の実装漏れの修正であり、公開contractの追加ではない。replacement/insertedの明示対応は維持する。

実VMB入力の原本・hash・byte lengthを固定したまま本文の1 byteを変更する再現で、修正前は正常・改変入力ともexit 0、修正後は正常入力exit 0、改変入力exit 1 / `P1102`となった。証跡は`/private/tmp/typaxis-legacy-source-identity-probe/`。共通fixtureの未記載mapping区間は同じ原本のidentityとして補い、本文・数式・出典bytesを保持した。VMBのinline/collapsible/appendixの公開buildは3件成功し、PDF bytesは修正前と一致した。ログは`/private/tmp/vmb-legacy-source-integrity-public.log`。この修正は全巻・book-2公開・原ノ味全巻の受け入れ完了を意味しない。

同修正後のCLI全体試験は413件成功・外部試験4件保留（274.03秒）。5,000 distinct SVGと5,000 alias配置の既存試験も成功した。ログは`/private/tmp/typaxis-host-source-common-cli-full.log`。

### 14.103 VMBの入力snapshot合計件数

VMBのcommand adapterに残った8,256 files / 16,512 entriesの上限は、既定8,192画像に加え、許可済み8,192 formal sourceと4,096 companion HTMLを含む入力を拒否していた。実ファイルで再現し、file上限を各種別の上限と従来のpackage/font/sidecar用64 slotsの和（20,544）、directory entry上限をその2倍（41,088）にした。formal/companionの件数定数は各encoderとsnapshotで共有する。2 GiB合計bytes、各file bytes、depth、symlink/special-file拒否を維持する。最大件数の全ファイルを照合し、1件追加で拒否する回帰試験を追加した。これは入力保全処理の境界試験であり、全数の意味内容や全巻PDFの成功証拠ではない。

修正後のVMB `go test ./internal/rendertypaxis -count=1`は成功（122.224秒、`/private/tmp/vmb-combined-snapshot-after.log`）。外部CLI用環境変数を設定しない通常回帰であり、§14.102の実CLI/PDF検証とは別の実行である。

### 14.104 book-2原本からresource-set /3への接続

private `book-v2-staging` featureに、source-admitted bodyを所有したまま実資源を読むCLI内部経路を追加した。machine-profileの資源ポリシーはその原本入力fingerprint、package hash、全M4有効上限とresource-set `/3`を結び、別ownerへの差し替えを拒否する。これは資源宣言のポリシーであり、公開production-book-2やlayout/profile全体の成功receiptではない。

原本の読み取り台帳に全resource candidateを先に登録し、同じhost処理でTT/TTC/CFF `/2`と画像をadmitする。base/拡張config上限の不一致はresource登録前に拒否し、失敗時も原本・candidate/opened identityと対象font/image IDを保持する。実原ノ味（SHA-256 `66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717`、23,060 glyphs）・TTC・PNGが、このbook-2経路で実admissionを通過した。本文flowのsolution kindも保持した。

接続7 tests成功（原ノ味の明示外部file試験を含む、1.29秒、`/private/tmp/typaxis-book-v2-resource-bridge-verified.log`）。既存machine-profileは50 unit + 1 compile-fail成功、capabilities関連4 tests成功。初回接続fixtureではTTC宣言の実file配置が欠けていたため失敗し、実TTCを配置して全5候補／5 opened identitiesを検証した。公開schema/contract/CLI commandは追加していない。book-2 shaping・layout・PDF・元全巻の受け入れは未完である。

### 14.105 book-2本文・生成番号の実shaping

private featureで、原本とresource-set /3を結んだ本文flowから、既存と共通のbidi・grapheme・段落context・glyph検証を実行する経路を追加した。結果は専用`BookV2AuthoredTextShape`で保持し、旧profileの成功receiptを発行しない。全宣言faceのcanonical instance表を所有し、実資源のmedia・URI・family・face index・任意宣言hashを照合する。別原本owner・別flow・同hash別resource ledger・異なるlimits/epochへの交換を拒否する。資源ポリシーの実装はsyntaxへ移し、machine-profileは同じAPIを再exportする。下位shapingからmachine-profileへの依存を作らない。

本文、リスト番号、脚注番号、指定改行contextでのreshapeを実行した。原ノ味の日本語と既存IVSを元のCFF /2でshapeし、実cmapのGIDとsource clusterが一致した。未収録IVSとIVS途中の改行を拒否し、出力record上限のちょうど境界と1件不足も確認した。接続試験11件成功（原ノ味を明示した外部file試験を含む、2.12秒、`/private/tmp/typaxis-book-v2-body-shape-tests-4.log`）。既存body回帰49件、authored-text回帰6件、shaping 24 unit + 1 compile-fail（外部2件はこのcrate実行では保留）、capabilities 4件が成功した。ログは`/private/tmp/typaxis-book-v2-body-shape-legacy.log`、`/private/tmp/typaxis-book-v2-shape-authored-legacy.log`、`/private/tmp/typaxis-book-v2-shaping-regression.log`、`/private/tmp/typaxis-book-v2-shaping-capabilities.log`。

これは本文shapeまでの接続であり、次期layout・pagination・PDF・元全巻の受入ではない。公開contract/schemaは引き続き未登録である。

### 14.106 book-2本文の実改行・glyph配置

本文inlineの準備と選択行へのprojectionを共通計算へ分離し、book-2専用の本文inline/line ownerを追加した。実shapingのclusterからUnicode改行用unitを作り、選択後は同じglyph参照・source/generated span・advance・offsetを保持する。全paragraphの幅の個数、owner、epochと候補探索の共通予算を検証する。行から得た実byte境界を同じcontext生成処理でreshapeへ戻す。

接続12 testsが成功（2.08秒、`/private/tmp/typaxis-book-v2-inline-tests-4.log`）。欧文の実glyph幅から複数行を選択し、再shape後の改行一致、glyph参照の同一性、本文全bytesの保存、候補探索の上限ちょうど／1不足、異なるowner/epoch・幅個数の拒否を確認した。元の原ノ味による日本語・IVSの実改行でも全source bytesと元GIDを保持し、IVSを途中で切らない。脚注参照の生成文字列はGenerated namespaceのまま選択行へ渡った。最初の欧文fixtureの固定幅では単語が収まらずNoFeasibleLineだったため、実advanceから幅を求めた。layout全体は69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-inline-layout-regression.log`）。

このownerは本文paragraphの行配置を表す。全source flowを保持するが、book-2のインラインvector/native binding、block/container/list/table/footnoteのページ配置、全体の収束とPDF出力はまだ接続していない。これらを省略した全巻成功receiptは発行しない。公開contract/schemaの登録も変更していない。

同じ共通計算の変更後、既存本文/PDF回帰49件も成功（259.53秒、`/private/tmp/typaxis-book-v2-inline-legacy-body.log`）。5,000 distinct SVG、5,000 alias配置、65,535を超えるglyph occurrenceを含む。これは既存production-book-1の回帰であり、次期全巻PDFの証明ではない。

### 14.107 book-2数式SVG・vectorの実資源bindingと混在行

原本所有者とresource-set /3を借用する`BookV2VectorBindings`を追加し、inline_vector・math_vector・vector_figure・math_vector_blockを実資源へ結んだ。旧bindingの資源照合・寸法/baseline/一様scale計算を共通helperへ分離した。新bindingはpolicy/input/package・全有効上限・実資源台帳をepochへ含め、各nodeの実IRと配置値をfingerprintへ含める。TeX、Alt/ActualText、言語、式番号、producer provenanceは原本側の専用型を借用し、1.4のvector/math receiptへ変換しない。

本文inline準備にこのbindingを接続した。本文glyphと2種類のinline vectorは同じ改行候補と行baselineを使い、選択したvectorが元bindingのfingerprintに一致する。混在行から得た実context境界を本文reshapeへ戻せる。block vector 2種類はgeometryまでであり、次期ページ配置の成功は発行しない。

接続16 tests成功（2.02秒、`/private/tmp/typaxis-book-v2-vector-tests-3.log`）。4種類を1つの実SVG資源へ結ぶこと、本文/atomの順序、TeXとAlt fallbackの区別、明示ActualText・言語・式番号の原本保持、別原本/資源/owner拒否、非一様scale拒否、4 receipt上限ちょうどと3での拒否を確認した。実Safe-SVG 1は通常inline vectorのbindingを通過する一方、math_vectorではSafe-SVG 2必須条件により対象nodeで拒否した。最初のfixtureはTeX/式番号slice用のidentity mapping区間を欠いていたため拒否され、原本文字列を変えずに正確な区間へ分けた。ActualText未指定時の期待値は既存規則どおりAlt fallbackとし、TeXとは別に検証した。共通helper変更後のlayout回帰は69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-vector-layout-regression.log`）。

native mathの次期binding、container/list/table/footnoteを含むページ組版、全体の収束、次期PDF/全巻/原ノ味全巻の受入は未完である。公開schema/contractの登録は変更していない。

既存CLIの公開precomposed-vector回帰も4件成功（1.73秒、`/private/tmp/typaxis-book-v2-vector-public-regression.log`）。独立PDF外部検査1件は明示環境を設定しないこの実行では保留であり、今回のPDF/UA合格としては数えない。

### 14.108 book-2 native mathの実計算と行配置

book-2の原本math node・computed style・resource-set /3の実フォントからnative計算を作り、本文と同じ改行/行配置へ接続した。計算本体とdisplayの自然寸法/基準線の算出は既存処理を共有する。専用`BookV2NativeMath`は原本、font instance表、実MATH faceのhash/index、計算結果を保持し、別binding ownerや資源/上限への交換を拒否する。旧`ValidatedMathReceipt`は発行しない。nativeのない本文には空の計算成功receiptを作らない。

全nodeの必要計算量とrecord/spoolをfont解析前にpreflightし、canonical font instance表の保持/一時領域も加算する。既存と同じ累積予算helperを使用し、先行消費量を含めて判定する。inline準備では所有するcontextと借用contextを扱い、reshapeごとに同じ計算を借用できる。選択行は専用のnative projectionを保持し、実source span・計算receipt・行baselineを結ぶ。display mathは自然geometryまでであり、次期ページ配置の受入ではない。既存の公開display経路は次期projectionを受け取った場合に拒否する。

接続18 tests成功（2.14秒、`/private/tmp/typaxis-book-v2-native-tests-3.log`）。実MATHフォント、inline/displayの2計算、shapeと同一canonical instance表、本文/native混在行、計算ownerの同一性、実境界によるreshape、所有/借用contextの一致、異なるbinding ownerと欠けたcontextの拒否を確認した。計算量・record・spoolの上限ちょうど/1不足と、MATHのない実フォントを選んだ場合の対象node付き拒否も確認した。計算量上限のfixtureは最初にhost configの拡張上限が揃わず資源受入前に拒否されたため、試験用host configにも同じ拡張上限を渡した。公開config/contractを新規発行したものではない。

共有処理の変更後、layoutは69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-native-layout-regression.log`）、既存CLI native回帰21件成功（0.99秒、`/private/tmp/typaxis-book-v2-native-legacy-regression.log`）。後者は既存production-book-1のnative PDF/font/Formula構造までを含む。book-2のcontainer/list/table/footnote/page組版、全体の収束、PDF/元全巻/原ノ味全巻と最終受入は引き続き未完である。

### 14.109 book-2本文フレームから実行幅への接続

本文フレーム計算を閉じたsource-flow adapter経由で共有し、`BookV2BodyInlineFrames`と`layout_book_v2_body_inline_lines`を追加した。12種類のsemantic containerの字下げ、実shaping済みlist markerの最大advanceとgap、fixed/fraction table列とcell幅、実footnote markerの列幅を同じ計算で解決する。選択行は専用frame ownerを保持し、body矩形・原本/shape owner・計算結果のfingerprintを結ぶ。frameのrecord消費を選択行の予算へ加算し、selected-line contextもframe込みのfingerprintを参照する。既存frameの識別子と出力ownerは維持し、次期には`typaxis.book-2-body-frames/1`を使用する。

脚注がある場合は元のbook-2 wireの単一default masterから最大脚注領域を取得する。入力bodyとの不一致、領域の欠落、実marker幅を差し引いた本文幅の枯渇は対象node付きで拒否する。呼出し側のbodyだけで脚注領域を補うことはしない。脚注の選択高さ・ページ割当や表の行高/ページ分割は後続処理である。

原ノ味を含む接続21 tests成功（2.03秒、`/private/tmp/typaxis-book-v2-frames-tests-mixed.log`）。12種類すべての入れ子字下げ、専用owner/body交換の拒否、幅枯渇、fixed/fraction列幅、実list/footnote marker列、脚注領域の欠落/幅不足、frame recordの加算とcontext fingerprintを検証した。実SVG/native混在行でもframe経由の選択結果を検査し、元binding/計算receipt・実baseline・source textを保持することを確認した。layoutは69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-frames-layout-regression.log`）。既存CLIの表回帰21件（7.19秒、`/private/tmp/typaxis-book-v2-frames-table-regression.log`）と脚注frame回帰1件（0.45秒、`/private/tmp/typaxis-book-v2-frames-footnote-regression.log`）も成功した。

次期の実行幅が解決された段階であり、全体のreshape収束owner、block/figure/table-row/footnote/page配置、PDF/元全巻/原ノ味全巻と最終受入は未完である。

### 14.110 book-2の実改行・再shaping収束owner

`with_converged_book_v2_body_lines`とnative context借用版を追加した。初回の本文shaping/実フレーム/行選択から実際の改行境界を取り出し、その境界で再shapingして同じ計算へ戻す。既存の`LineReshapeFeedback`と比較stateのcanonical化を共有し、全選択結果が安定した場合だけ専用`BookV2ConvergedBodyLines`をcallbackへ渡す。呼出し側から安定フラグや完了receiptを渡すAPIは設けていない。最終shape・inline・frameの借用graphはcallbackの範囲内に保持する。

native計算は初回に一度作るか、上位ownerから同じimmutable contextを借用し、全反復で再利用する。候補探索上限は初回と全反復で合算する。reshape pass上限は既存のone-shot permitで実作業前に消費し、未収束のまま尽きた場合はcallbackを呼ばない。各段階のfragment上限は維持しているが、後段を含む全体のallocation/page/generated-reference収束は別途必要である。

最終接続23 tests成功（2.57秒、`/private/tmp/typaxis-book-v2-reshape-tests-final.log`）。実際の非安定→安定の連続pass、最終context付きshape、callback一回のみ、全候補数の上限ちょうど/1不足、1 passで未収束の場合の拒否を確認した。表/list/footnoteの実幅、SVG occurrence、native計算receiptとbaselineを最終callback内でも検証した。layout回帰69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-reshape-layout-regression.log`）、共有state helperを通る既存の脚注生成glyph/収束試験も成功した（0.35秒、`/private/tmp/typaxis-book-v2-reshape-legacy-regression.log`）。book-2のblock/figure/table-row/footnote/page配置、最終PDF/全巻/公開受入は引き続き未完である。

### 14.111 book-2脚注参照と最終選択行の対応

専用`BookV2FootnoteLines`を追加し、収束ownerが最終選択行とともに保持するようにした。定義・参照の収集と生成marker全文字のcoverage検証は既存処理を共有する。共通source flowが保持する元の定義IDを借用し、定義の出現順による番号、event/paragraph範囲、本文内または他定義内からの参照、最初と最後の実glyph cluster位置を結ぶ。IDの辞書順と定義順が違う場合だけ並べ替え索引を持つ。

record予算には選択行、全定義、必要なID索引、全参照とcoverage領域を合算する。専用ownerは選択行pointerと全resource limitsを検証し、別の行配置や上限への交換を拒否する。脚注のページ需要・領域分割・ページ配置はまだ後続段階である。

脚注の接続試験では、ID順が逆の12定義、本文からの13参照と定義内からの1参照、2桁番号、同じ定義への重複参照、元ID文字列の同一性、event/paragraph範囲、実cluster位置を確認した。選択行と索引の合計record上限ちょうど/1不足、別行owner/別上限の拒否、収束callback内の同一対応も確認した。連続参照の初期fixtureは改行不能な箇所が10,000,000 rawの幅を超えて拒否されたため、対応関係を試験する本文幅を30,000,000 rawへ変更した。既存脚注回帰86件も成功した（9.42秒、`/private/tmp/typaxis-book-v2-footnote-legacy-regression.log`）。

### 14.112 book-2通常図版の実寸法とキャプション

通常figureの準備を共通source/image adapter経由でbook-2へ接続した。実resource-set /3のPNG/JPEGからpixel比に沿った物理寸法を求め、SVGから実際のintrinsic寸法と一様16.16 scaleを求める。SVGは既存と同じ幅を超えない量子化とties-even寸法計算を使用する。raster幅の未指定や表現できないSVG scaleは対象node付きで拒否する。

専用inline ownerが元のfigure、source index、実image hash、media/dimensionsを保持し、captionは元のparagraph/glyphフローを通る。準備と行選択でfigureのrecordを計上し、再shapingの最終ownerにも同じ原本と寸法が残る。通常figureの自然寸法準備であり、次期ページ上のfigure配置やPDF paintはまだ完了していない。

脚注・通常図版を含む最終接続27 tests成功（2.07秒、`/private/tmp/typaxis-book-v2-footnote-figure-tests-3.log`）。実PNG/JPEGの2:1比、SVGのintrinsic寸法と量子化した一様scale、元caption paragraph、最終収束時のsource/寸法保持、raster幅未指定と小さすぎるSVG scaleの拒否を確認した。試験用SVG宣言のprovenance不足とraster宣言への不要なprovenanceを修正し、`svg-safe-2`だけがその必須フィールドを持つ既存decode規則を維持した。

共通処理の回帰はlayout 69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-footnote-figure-layout-regression.log`）、既存図版7件成功（0.72秒、`/private/tmp/typaxis-book-v2-figure-legacy-regression.log`）。後者は既存production-book-1の通常SVG FigureのPDF/manifestも含む。book-2のblock/vector/table-row/footnote/page配置、全体のallocation/page収束、PDF/元全巻/原ノ味全巻と最終受入はまだ未完である。

### 14.113 book-2独立vectorブロックと実式番号

式番号のUnicode itemization・linked shaper・glyph/cluster検証を既存処理から共有化し、専用`BookV2EquationNumberShapes`を追加した。最終本文shapeと同じresource-set /3のcanonical font instancesを参照し、原文の番号、実フォント、source span、言語、実advanceとline-heightを保持する。CFF /2のcoverage検証を通し、折返しを必要とする文字や不足したcomputed text styleを拒否する。番号のない原本には空の成功receiptを発行しない。既存の式番号receiptへ変換するAPIはない。

retained recordは先行消費量・集合・番号・run・glyph・clusterを合算し、backendの一時領域は既存のshaping context上限で別途制限する。glyph canonical化の一時領域も事前に制限する。専用集合は先行消費量と自身の保持量を分けて記録し、後続段階が本文と式番号を同時に保持する場合に計上漏れが起きないようにした。

`BookV2VectorBlockLayout`は実選択行のframeとbindingを参照し、入れ子container/list/table/footnoteの本文幅にblock自身の字下げを適用する。水平配置、式番号の右端位置・最小間隔、数式と番号の高さ合わせは既存geometry処理を共有する。原本のviewportとscaleを保持し、番号との衝突は数式を動かしたり縮小したりして隠さず拒否する。source event位置、原本binding、番号shapeを保持し、page name・keep・spacing等は元binding/flowに残る。ページの選択や最終paintを行う段階ではない。

最終接続32 tests成功（2.13秒、`/private/tmp/typaxis-book-v2-block-tests-final.log`）。実原文`(1)`のglyph/source対応、本文と同じfont instance、原ノ味CFF font、式番号集合の先行消費量を含む上限ちょうど/1不足、style不足と番号のない入力を検証した。ブロックでは実container frameと自身の字下げ、start/center/end配置、式番号の右端・最小gap・高さ合わせ、番号との衝突拒否、frame/number owner不足、record上限ちょうど/1不足を検証した。最終収束callbackで作った式番号集合とblock layoutでも、同じbindingと寸法および合計保持量が維持されることを確認した。

共有処理の回帰はshaping 24 unit + 1 compile-fail成功（外部fixture指定2件はignored、`/private/tmp/typaxis-book-v2-equation-shaping-regression.log`）、layout 69 unit + 1 compile-fail成功（`/private/tmp/typaxis-book-v2-block-layout-regression.log`）。既存common driverは番号付き/なしPDFの反復生成を含む2件成功（0.65秒、`/private/tmp/typaxis-book-v2-block-legacy-regression.log`）、opt-inの独立検証1件はignoredである。これは次期PDF/UA受入の証拠ではない。次期の本文/表行/脚注/図版をまとめるpage配置、全体allocation/page収束、最終PDF/元全巻/原ノ味全巻・公開受入は引き続き未完である。

### 14.114 book-2本文・定義streamの共通収集と表行計測

`BookV2PreparedBodyFlow`を追加し、最終選択行・専用vector block・脚注line ownerを同じ共通collectorへ接続した。閉じた借用adapterが共通のsource event、実選択行、実図版/native math寸法、container/list/table frameを渡す。旧source/layout/page receiptへの変換は行わない。元のsource順で段落の各行、独立数式、図版、強制改ページ、list marker、脚注定義の範囲と参照先を保持する。名前付きページはこの段階でも明示的に未対応として拒否する。本文と脚注定義のleafを混ぜず、実reference clusterをそれぞれの局所item位置へ結ぶ。

list markerと脚注番号は、同じ最初の実paint itemへ既存のleading/trailing計算を適用する。book-2 native mathも実内容として扱う。数式から始まる定義ではviewport top offsetと実baselineを用いる。vector blockのpage/spacing/keep/baselineは元bindingから取得する。block集合も先行消費量と自身・番号の保持量を分離し、独立に準備した脚注とblock/numberの合計をmax-of-totalsで過小計上しない。新たなbody ownerと収集leaf・marker・reference・table境界を同じrecord上限へ加算する。

`BookV2TableMeasurements`は専用body ownerを保持し、既存のセル内容・入れ子表・rowspan不足分・row band計測を共有する。セル中の入れ子表は一つの子の高さであり、その並列セルの高さを合算しない。原本のtable/cell境界・source index・spacing・keepを保持する。専用fingerprintのalgorithmは`typaxis.book-2-table-measurements/1`、一時canonical bufferも割当て前にspool上限で制限する。旧table measurementのfingerprint headerと予算・計測順は維持する。これは表の実高さの計測までであり、表分割候補・繰返しheader・脚注予約・ページ位置・PDF paintを許可するownerではない。

最終接続34 tests成功（2.13秒、`/private/tmp/typaxis-book-v2-body-table-tests-final.log`）。既存の原ノ味/source/resource/shape試験に加え、実最終行からのblock/figure/native math/脚注stream、owner置換拒否、body record上限ちょうど/1不足、入れ子表とrowspan、表計測の累積上限と反復fingerprintを検証した。同じ元source/SVGの結合fixtureで、式番号付き表内数式・通常SVG図版とcaption・vector figureから始まるlist・数式から始まる脚注を一度に収集・計測した。段落を1行と仮定した試験は実選択行数に合わせて修正し、入れ子セルの狭すぎるpositive fixtureは外側の宣言幅を広げた。通常figureの必須placementも既存wire仕様どおり補った。parserや改行・配置規則は緩和していない。

ローカル回帰: `cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination --features book-v2-staging --locked --target-dir /private/tmp/typaxis-vmb-book-build` は92件成功（0.21秒、`/private/tmp/typaxis-book-v2-body-pagination-regression.log`）。同じmanifest/target-dirのdefault CLI `table` filterは49件成功（9.71秒、`/private/tmp/typaxis-book-v2-body-table-legacy-regression.log`）、`footnote` filterは86件成功（11.43秒、`/private/tmp/typaxis-book-v2-body-footnote-legacy-regression.log`）。feature CLI checkと`git diff --check`も成功。次期の表分割・本文/脚注の共通page選択と収束、構造/PDF、正式元全巻export、TrueType/原ノ味全巻・管理host計測・独立/人手受入・公開ゲートは引き続き未完である。

### 14.115 book-2の表分割cursorと見出し行の予約

表のcapacity searchをprivate `TableBreakKernel`へ共有化し、`BookV2TableBreakSearch`・`BookV2TableCursor`・`BookV2TableFragmentSelection`を追加した。専用cursorは正確な`BookV2TableMeasurements`とtable indexを保持し、別の同内容measurementのcursorや終端cursorを拒否する。旧cursor/resultへの変換は行わない。共通kernelは実セルの切断禁止区間、keep、rowspanの終了位置index、見出し行の高さを用いて、与えた高さ内の最後の安全な共通境界を選ぶ。小さい領域で入らなければ候補なし、宣言された最大領域でも入らなければOversizeとする。

本文中の表には実body frame、脚注定義中の表には宣言されたfootnote regionの高さを用いる。各分割でheadを再予約し、source上のhead occurrenceは初回のみsemantic leaf rangeへ含める。繰返しheadの配置候補には専用flagを付ける。原本item indexとcell owner、分割内のtopを保ち、本文セルの実内容を重複・脱落なく次のcursorへ渡す。leaf範囲と配置候補の導出も旧ownerと共有するが、この結果にpage indexやPDF paint権限はない。

新しいfragment fingerprintは`typaxis.book-2-table-fragment/1`を使う。先行measurement・外側の先行消費量、探索index、begin/evaluate、cell sliceとresultを一つのrecord上限で保持し、再試行や捨てた候補でも消費量を戻さない。比較sort・境界探索・cell treeの走査も同じwork上限に加算し、canonical bufferは割当て前にspool上限で制限する。旧algorithm/headerと計測fingerprintのencoding順は維持した。

この接続は表の分割候補までである。入れ子表の再帰分割とセル内の強制改ページは既存の明示的な未対応診断を維持しており、本文/表/脚注の共通ページ選択・予約・収束、最終構造/PDF、全巻と公開受入は引き続き未完である。

最終接続36 tests成功（2.19秒、`/private/tmp/typaxis-book-v2-table-break-tests-final3.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。実最終行からheader/rowspanを複数fragmentへ進め、セル内容の重複・脱落なし、繰返しheadのflagとsemantic除外、capacity再試行、終端/同内容別owner/負・過大capacityの拒否、record/work上限ちょうどと1不足、反復fingerprint、高さゼロの行の構造的な前進、脚注の実region上限とhead oversizeを検証した。脚注番号列を含む成功fixtureは固定列幅をそのまま流用せず、残り幅で2列へ配分する宣言へ修正した。式番号付き数式・通常SVG/captionを含む共通fixtureでも、安全でない高さでは候補なしとなり、全高さで実cell leafが一度ずつ選択される。

旧形式の最終table回帰49 tests成功（9.00秒、`/private/tmp/typaxis-book-v2-table-break-legacy-final.log`）。コマンドは同じmanifest/target-dirの`cargo test -p typaxis-cli --bin typaxis table --locked`。表分割と脚注予算の共有、header/source/PDFへの既存接続も対象である。pagination feature回帰92 tests成功（0.15秒、`/private/tmp/typaxis-book-v2-table-break-pagination-regression.log`）。feature CLI checkと`git diff --check`成功。reviewで専用fragmentの終了位置の二重保持を除き、最終36件はその変更後の結果である。これらは次期の最終ページ・PDF/UA・元全巻受入の証明ではない。

### 14.116 book-2脚注定義の分割候補と継続

`BookV2FootnoteBreakSearch`・`BookV2FootnoteCursor`・`BookV2FootnoteFragmentSelection`を追加した。実body flowを借用し、定義内の局所item位置、選択した実leaf、脚注番号、実参照clusterに対応するoccurrence、候補一覧と採択位置、消費した強制改ページ、継続cursorを保持する。旧形式のflow/cursor/selectionへの変換はない。段落の実際の行数とheading context、定義内keep検証、capacity選択は共通処理を使う。借用型`FootnoteSearchKernel`は既存searchが持つ同じcharge/step counterを直接更新するため、旧ページ/表/脚注候補間の累積予算の受け渡しは変更していない。

先行flowと外側の先行消費量、search/context/cursor、再試行を含むcandidate record、実item走査を累積上限で制限する。最大高さは元の宣言footnote regionから取得する。小さいcapacityで入らなければ候補なし、最大capacityでも入らなければOversizeである。定義の先頭が強制改ページなら実内容も脚注番号も消費せずにそのeventだけを進め、最初の実内容を選んだfragmentへだけ番号を付ける。keepが定義内の強制改ページを越える場合は拒否し、定義末尾のkeepを次の定義へ接続しない。

参照の局所item範囲検索も既存処理と共有化した。各fragmentは実際に選択した行と交差するoccurrenceのみ返し、定義内の参照をbody streamへ混ぜない。表を含む定義は並列cell streamを通常の行列に平坦化せず、`table_footnote_definition`として専用の共通選択処理への接続が必要であることを明示する。この段階は一つの定義のcontent候補までであり、定義間の間隔・separator、複数定義の需要/予約、参照ページとの結合、page index・paint権限はまだ付与しない。

最終接続37 tests成功（2.11秒、`/private/tmp/typaxis-book-v2-footnote-break-tests-final.log`）。コマンドは§14.115と同じoriginal Harano環境指定付きfeature CLI `book_v2_resources -- --include-ignored`。12定義・反復/複数桁番号・定義間参照に対する局所選択、marker/leafの元owner、候補再試行、record/work上限ちょうどと1不足、同内容別body ownerのcursor拒否を検証した。強制改ページ前後の継続、番号の一度限りの配置、参照occurrenceの保持、keep矛盾とoversize拒否を追加検証した。共通の実SVG/式番号/table fixtureにある数式から始まる定義でも、数式と後続段落を別fragmentへ選び、継続側へ番号を再付与しないことを確認した。表定義の平坦化拒否も検証した。

default CLI `footnote`回帰86 tests成功（10.49秒、`/private/tmp/typaxis-book-v2-footnote-break-legacy-regression.log`）、pagination feature回帰92 tests成功（0.15秒、`/private/tmp/typaxis-book-v2-footnote-break-pagination-regression.log`）。manifest/target-dir/locked指定は§14.115と同じ。feature CLI checkと`git diff --check`も成功。次期の複数定義需要/予約と本文/表/脚注の共通page選択・収束、最終構造/PDF、正式元全巻export、TrueType/原ノ味全巻・管理host計測・独立/人手受入・公開ゲートは引き続き未完である。

### 14.117 book-2の脚注需要・分岐状態・継続の管理

`BookV2FootnoteDemandSearch`・`BookV2FootnoteDemandState`・`BookV2FootnoteDemandSelection`を追加した。本文の候補範囲に実際に含まれる参照から、定義をUnreferenced/Pending/Completeとして管理する。重複参照はpending queueへ重ねて登録せず、最初の参照ownerを保持する。選択した脚注fragment内に別定義への参照があれば、その実occurrenceから需要を追加する。途中の定義はqueue先頭で継続し、完了した定義を再参照しても再配置候補へ戻さない。

状態は元body flow、search owner ID、branch state IDに結び付くimmutable snapshotである。候補を試しても入力状態を変更しない。同内容のcursorを持つ別branchや別searchのselectionを消費することはできず、採択した元branchから新しい状態を作る。cursor自体はbook-2専用型のままであり、旧形式の状態・選択結果を発行しない。

snapshotの作成・複製、first-reference登録と重複抑制、継続/完了更新とqueue移動はprivateな共通kernelへ抽出した。旧cursorとbook-2 cursorを異なる型引数として保持し、閉じたcontent adapterを通して各searchの同じrecord/work counterを更新する。先行flow/外側保持量、snapshot/queue、content候補と再試行、状態複製・参照検索・pending移動の実作業を同じ累積上限で制限する。旧形式のpage/table/footnoteの予算受け渡しと状態所有検証は維持した。

本文範囲の需要追加は改ページの正当性を認証する段階ではない。範囲が表cell streamと交差する場合は`table_body_demand`として拒否し、表分割のsemantic leafを使う後続の共通選択処理へ接続する必要があることを明示する。複数定義を同じfootnote regionへ予約する処理、separator/定義間隔、表候補との予算共有、本文/表/脚注のpage選択と収束、最終構造/PDF、全巻・公開受入は引き続き未完である。

最終接続38 tests成功（2.82秒、`/private/tmp/typaxis-book-v2-demand-tests-final.log`）。コマンドは§14.115と同じoriginal Harano環境指定付きfeature CLI `book_v2_resources -- --include-ignored`。重複参照、最初の参照owner、immutableな分岐、別branch/searchのselection拒否、途中の定義を先頭で続けること、定義内参照の追加、完了定義の再参照、累積record/work上限ちょうどと1不足を確認した。実SVG数式から始まる定義の需要・継続、および表と交差する本文範囲の拒否も最終callback内で検証した。

default CLI `footnote`回帰86 tests成功（12.16秒、`/private/tmp/typaxis-book-v2-demand-legacy-regression.log`）、pagination feature回帰92 tests成功（0.17秒、`/private/tmp/typaxis-book-v2-demand-pagination-regression.log`）。manifest/target-dir/locked指定は§14.115と同じ。公開book-2ゲートと全巻受入はまだ完了していない。

### 14.118 book-2複数脚注の領域選択と各参照先の先頭予約

`BookV2FootnoteRegionFragment`・`BookV2FootnoteRegionSelection`を追加した。各fragmentは実定義contentの専用selectionと領域内のoffsetを保持し、領域全体は元search/branchに結び付く。定義間の余白は直前の実itemのafterと次の実itemのbeforeを加算する。最初の定義に前ページの余白を持ち越さず、強制改ページは領域の終端とする。使用高さ・与えられた高さ・強制改ページowner・次のimmutable需要状態を返す。

順番に内容を収める`select_region`と、開始時にpendingだった各定義へ合法な先頭fragmentを確保する`select_required_region`を接続した。後者は各定義の最小候補と後続に必要な高さ・余白を先に予約し、その残りで前の定義の候補を評価する。長い最初の脚注が後の参照先を押し出す候補は採らない。各定義は途中でも次の定義へ進め、継続する定義は次の状態のqueueに残す。選択した内容から新しく生じた定義内参照は次の状態へ明示的に追加し、開始時に存在した需要と取り違えない。後続定義が必要な場所で強制改ページを越えることはできない。

領域選択と先頭予約の二つの算法は、それぞれprivateな共通kernelへ抽出した。旧形式とbook-2は異なるcursor/state/selection型を保持したまま、同じ実itemとparagraph/heading context、元のcharge/work counterを渡す。候補の最小高さ、suffix予約、実容量に対するcost再評価、状態更新の順序と累積予算を維持する。旧receiptへの変換はない。

このownerは脚注内容領域の候補である。separator帯、本文・表との衝突と同一ページ上の参照先開始、ページ選択の全体収束、page indexとpaint権限は後続の結合処理で扱う。表を含む定義と表cell由来需要の接続、最終構造/PDF、正式元全巻exportとTrueType/原ノ味全巻・管理host計測・独立/人手受入・公開ゲートも引き続き未完である。

最終接続40 tests成功（2.32秒、`/private/tmp/typaxis-book-v2-region-tests-final.log`）。コマンドは§14.115と同じoriginal Harano環境指定付きfeature CLI `book_v2_resources -- --include-ignored`。実定義間の非ゼロ余白、領域高さちょうど/1不足、途中の定義から次領域へ進めること、別branchの拒否、累積record/work上限ちょうど/1不足を確認した。先頭予約では長い定義と短い定義の両方を開始し、継続と完了が混在するqueueを保持する。開始時に未参照だった定義内参照は次の状態へ残る。強制改ページが後続定義を妨げる候補の拒否、最後の定義の強制改ページで停止する場合、次領域で実内容の番号を一度だけ配置することも検証した。

default CLI `footnote`回帰86 tests成功（14.99秒、`/private/tmp/typaxis-book-v2-region-legacy-regression.log`）。先頭予約、表・本文との脚注予算共有、separatorと既存PDF出力を含む。これは次期の最終ページ/PDF/全巻受入の証拠ではない。

pagination feature回帰92 tests成功（0.17秒、`/private/tmp/typaxis-book-v2-region-pagination-regression.log`）。コマンドは`cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination --features book-v2-staging --locked --target-dir /private/tmp/typaxis-vmb-book-build`。最終のfeature/default試験ビルドに警告・コンパイルエラーはなく、`git diff --check`も成功した。元の公開ゲートと最終受入条件は変更していない。

### 14.119 book-2本文候補と脚注予約の同時fit

`BookV2BodyFootnoteCandidate`を追加した。実本文leaf範囲の高さ・隣接余白・keep・強制改ページ・参照clusterの完全な包含を検査し、その候補で必要になる定義を脚注領域へ予約する。元のbody/footnote frameから水平の重なりと使用高さを計算し、separator帯を含む予約が本文と衝突する候補を採らない。脚注領域が本文の横や上にある場合も元の矩形に従う。実内容のない強制fragmentだけではseparator矩形を発行しない。

新しい需要の追加、全定義の先頭予約、選択内容にある未開始の定義内参照の拒否、領域下端への整列はprivate共通fit kernelで行う。本文候補の高さと参照範囲検査も共通化し、旧形式の候補・表・mixed-page処理とbook-2は同じ算法に異なる専用ownerを渡す。候補は元search/branchに結び付き、別分岐への流用を拒否する。落選や再試行でrecord/work予算は戻さない。

book-2の表範囲検査は実収集表を上限付きで走査し、cell streamと交差する通常本文候補を拒否する。表の直後のkeepは外側の表境界で判断し、最後の並列cellのkeepを次の本文段落へ流用しない。これは局所候補のfitであり、前後の本文cursor連続性・候補順位・widow/orphanを含むページ選択・複数ページの収束・最終paint権限は後続段階である。

### 14.120 book-2表候補と脚注継続の予算共有

`BookV2TableFootnoteSearch`・`BookV2TableFootnoteState`・`BookV2TableFootnoteSelection`を追加した。実表fragmentのsemantic leaf範囲に完全に含まれる参照だけを需要へ加え、§14.119の共通fitへ渡す。繰返しheaderはsemantic範囲へ再度入らないため、見出し内の参照や脚注番号を重複させない。表と脚注双方の継続状態を保持し、別search/branchの状態を拒否する。

表探索の残りrecord予算と累積workを脚注探索へそのまま渡し、その後の表begin/evaluateでは同じcounterを移して戻す。エラーや落選の経路でもcounterを戻し、別の満額予算を発行しない。表の大きい候補が脚注を収められなければ入力状態を保って候補なしを返し、小さい表候補で再試行できる。表を選ばず未完脚注だけを進める場合は表cursorを変更せず、表の終端後にも残る脚注を継続する。

表由来需要の抽出・完全な参照包含・予約fitも共通kernelへ移し、旧table/footnote接続とbook-2で共有する。表間の本文・表外余白・ページ順位をまとめるbook cursor、入れ子表の再帰分割と表を含む脚注定義、全体収束・構造/PDF、正式元全巻exportと各全巻/公開受入は未完である。ここでの表領域fitを最終ページownerや公開book-2対応として扱わない。

§14.119–14.120の最終接続44 tests成功（2.16秒、`/private/tmp/typaxis-book-v2-joint-tests-verified.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。本文では実高さに合わせた領域の衝突/非衝突、separatorを含む正確な矩形、横・上にある領域、keep/強制改ページ、別branch/search、空の本文候補での脚注継続、完了定義の再参照、未開始の定義内参照、累積record/work上限ちょうどと1不足を確認した。

表ではheader内参照と後続行の別定義、長い脚注の継続、表を止めて脚注だけを進める再試行、繰返しheaderのsemantic除外、番号を一度だけ付けること、反復fingerprint、同じmeasurementを使う別searchの拒否、累積record/work境界を検証した。表探索中のwork枯渇後にも同じcounterが戻り、再試行でrecordは増え続け、続く脚注beginも枯渇を検出することを確認した。表外のlist内参照は当該表fragmentの参照へ混ぜていない。

実SVGキャプション内参照・式番号付き表内数式・数式から始まる脚注も結合した。表が終端へ進んだ後、残る脚注段落だけを継続し、番号を重複させない。追加fixtureの参照spanは親caption paragraphの0..7へ修正し、原文包含検査による拒否を維持した。元の2,000,000 raw領域には実測上脚注全体が収まったため、継続を検査する宣言領域を1,500,000 rawへ変更した。元source bytes、数式geometry、page幅や検証算法を変更して成功させたものではない。

ローカル回帰: 同じmanifest/target-dir/locked指定のdefault CLI `footnote` filterは86件成功（10.68秒、`/private/tmp/typaxis-book-v2-joint-footnote-legacy-regression.log`）、`table` filterは49件成功（8.13秒、`/private/tmp/typaxis-book-v2-joint-table-legacy-regression.log`）。`cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination --features book-v2-staging --locked --target-dir /private/tmp/typaxis-vmb-book-build`は92件成功（0.16秒、`/private/tmp/typaxis-book-v2-joint-pagination-regression.log`）。feature CLI checkと`git diff --check`も成功。各最終試験ビルドにcompiler warning/errorはない。これらは次期のページ列・PDF/UA・元全巻受入の証明ではなく、元の完了条件は維持する。

### 14.121 表の原文所属・親子関係と空の子表の計測

共通table collectorが、各表の原文上の脚注定義index（本文ならNone）と親table indexを保持するようにした。本文末尾の空の表は脚注の先頭と同じleaf位置を持ち得るため、`items.start >= body_end`による所属判定を置き換えた。表検索の最大高さ、定義tableの拒否、本文tableの需要検証も実所属を使う。book-2の本文keep判定も、leaf範囲から親を推測せず実parentを使う。

空の子表は親cellのleaf cursorを進めないが、原文上は一つのtable occurrenceである。共通計測はchild cursorが残っている間も処理を続け、各子表の高さ・前後余白を一度だけ加算する。隣接する空の子表もchild順序で前進する。content recordの数は従来の実child数を含む式で事前に計上し、親owner・定義所属・範囲の整合を検証する。子表を高さのない本文leafへ置換したり、並列cellを直列に足したりしない。

これは空のtable topologyの所属と計測の修正であり、入れ子表の再帰fragment選択や最終paintを許可する変更ではない。旧入力の既存fingerprint header/encodingと非空tableの計測順は維持する。

### 14.122 book-2の本文・表を交互に含む候補と原文cursor

`prepare_book_v2_table_body_search`は全本文tableの実探索indexを一度準備し、その残りrecord/work予算を同じ脚注需要searchへ渡す。各table searchは予算を保持しない待機状態になり、begin/evaluate時に共通counterを移して戻す。集合準備の走査・割当て・定義table拒否は旧形式と共通のprivate kernelを使う。

`BookV2BodySourceState`は本文leaf位置、原文上の次のtable ordinal、必要なら実table continuation、需要状態を保持する。隣接する空の表は同じleaf位置でも異なるordinalを持つため、表を一つ消費するたびに構造的に前進する。`BookV2BodyCandidatePart`で指定した本文範囲と実表cursorを、`BookV2BodyMixedCandidate`へまとめる。表の飛ばし・二重消費、完了前の表に続く本文、継続cursorの取り違え、別branch/searchの状態を拒否する。

共通mixed kernelは実本文高さ、marker leading、table外余白、keep、表fragmentのsemantic参照を順に合成し、共通脚注fitへ渡す。結果は各partのtop/heightと実source selectionを保持する。部分表のcell leafを本文の連続消費範囲とみなさず、tableが終端へ達したときだけそのsource範囲の後へ進む。book-2では別ordinalを追跡し、旧形式の入口は既存の局所範囲契約を維持する。新しい状態・候補・次状態の保持量、全table探索と脚注需要の消費量を同じ累積上限で制限する。

ここで入力するrequestsは局所候補であり、自動的なpage boundary列挙・候補順位・widow/orphan・強制改ページのページ生成・複数ページの収束と配置は後続段階である。最終構造/PDF、入れ子表/表を含む脚注定義、正式元全巻exportと元の全巻/公開受入も引き続き未完である。

§14.121–14.122の最終接続47 tests成功（2.12秒、`/private/tmp/typaxis-book-v2-mixed-candidate-tests-final.log`）。コマンドは§14.120と同じoriginal Harano環境付きfeature CLI `book_v2_resources -- --include-ignored`。隣接/末尾の空表、実定義所属、空の子表二つの親cell計測、原文ordinalの連続性、table continuation/別branchの拒否、実SVG caption参照・式番号付き表内数式と数式から始まる脚注の混在、残る脚注だけの継続、累積record/work境界を検証した。

default CLI回帰は`table`49件（7.50秒、`/private/tmp/typaxis-book-v2-mixed-table-regression.log`）、`mixed`23件（2.76秒、`/private/tmp/typaxis-book-v2-mixed-legacy-final.log`）、`footnote`86件（14.75秒、`/private/tmp/typaxis-book-v2-mixed-footnote-final.log`）成功。pagination feature回帰92件（0.22秒、`/private/tmp/typaxis-book-v2-mixed-pagination-final.log`）成功。manifest/locked/target-dirは§14.120と同じ。最終ビルドに警告/コンパイルエラーはない。これらは後続の自動page選択を追加する前の接続・回帰証拠である。

### 14.123 book-2の自動ページ境界選択と原文順のページ列

`BookV2BodyMixedPageState`・`BookV2BodyMixedPageSelection`・`BookV2BodyMixedPageSequence`を追加した。実本文の段落境界と表の安全な各切断位置を列挙し、共通の段落分割cost（widow/orphan・heading contextを含む）で順位を付け、脚注fitを試す。最初に収まる候補を採り、同順位は列挙順を維持する。境界列挙自体が上限を超えた場合は途中の成功候補で終了しない。順位付けの保持・比較/移動と表の早い切断位置探索は旧形式とprivate kernelを共有する。

各状態は§14.122の専用source状態とpage indexを保持する。空表を消費するとtable ordinalで前進し、実本文位置が同じ隣接表を区別する。keepは直前に消費した原文occurrenceから判定し、同じleaf位置にある将来の空表のkeepを先取りしない。継続cursorはそのtable ordinalにだけ再利用する。強制改ページと末尾の空ページ、脚注だけの継続を明示的に保持し、ページ列全体で本文・表・脚注または要求された最後の空ページの前進を検証する。

page数・候補数・ページごとの脚注fit再試行数、準備/採択/破棄を含むrecordとworkに宣言された累積上限を適用する。各selectionは元branchを保持し、sequenceは実measurement fingerprintと発行searchに結び付く。ここで選ぶのはページ列であり、物理配置・収束・最終paint/構造/PDFのreceiptではない。

接続52 tests成功（4.00秒、`/private/tmp/typaxis-book-v2-auto-pages-tests-4.log`）。コマンドは§14.120と同じoriginal Harano指定feature CLI。複数ページの表cell内容の一度限りの消費、繰返しheaderのsemantic除外、連続空表/強制空ページ、sourceに沿ったkeep、immutable branch/別search拒否、脚注fit再試行と脚注だけのページ、実SVG/式番号/本文/脚注の自動ページ列、record/workの上限ちょうどと1不足、page/reflow/lookback上限を検証した。追加fixtureは渡す本文矩形をpage masterにも明示し、empty semantic containerの既存拒否条件を保つため実段落を含めた。lookback負例は境界が二つ以上列挙される宣言領域を使い、正常に一候補だけとなる小領域を失敗扱いしない。

旧形式のmixed回帰23件成功（1.28秒、`/private/tmp/typaxis-book-v2-auto-pages-legacy-mixed.log`）、footnote86件成功（10.87秒、`/private/tmp/typaxis-book-v2-auto-pages-legacy-footnote.log`）、table49件成功（8.04秒、`/private/tmp/typaxis-book-v2-auto-pages-legacy-table.log`）。feature CLI checkと`git diff --check`成功。最終構造/PDF、入れ子表の再帰fragment/表を含む定義、正式元全巻exportと原ノ味/TrueType全巻・公開/管理host/独立/人手受入は引き続き未完である。

§14.123のpagination feature回帰92件成功（0.20秒、`/private/tmp/typaxis-book-v2-auto-pages-pagination.log`）。コマンドは同じmanifest/target-dir/locked指定の`cargo test -p typaxis-pagination --features book-v2-staging`。このページ選択の最終試験ビルドに警告/コンパイルエラーはない。以後の物理配置変更の検証とは分けて記録する。

### 14.124 book-2の選択ページへの実本文・表・脚注配置

`BookV2BodyMixedPlacedPage`・`BookV2BodyMixedPlacedSequence`を追加した。元の専用page sequenceを借用し、本文領域の原点と各partのtop、表の実cell leafの相対位置、脚注領域のseparator帯/各定義offsetから物理座標を計算する。本文と定義内のitem indexを区別し、table cell ownerと繰返しheader flagを各fragmentへ対応付ける。並列cellの順序を通常段落列へ変換しない。

本文行・native display math・通常画像・数式vectorの配置は、既存のprivateなgeometry処理へ閉じた`BodyLines`/`BodyBlocks`を渡して共有する。実line baseline、native mathのbaseline/viewport、元vectorのviewport offset/寸法、figureの実寸法を使用する。sourceに結び付くpage ownerは専用型のままで、共通の座標値だけを返す。旧paint receiptへの変換はない。

領域内の余白/marker leadingを含む消費高さ照合、separator ink、実list/footnote番号の配置も`ContentPlacement`へ共有化した。list bindingは原本のglobal item indexを検索し、定義番号はその実先頭itemへ一度だけ配置する。右揃え幅・ascender/descender・baselineには実shaped markerの計測値を使う。paginationへshaping依存を追加せず、閉じたgeometry viewを介する。選択で消費した同じrecord/work counterを配置でも更新し、fragment・cell role・marker・separatorと列の割当てを上限内に保持する。

この結果は選択したページの物理geometryであり、page/reflow全体の収束、最終math terminal、構造/PDF・公開book-2 gateを認証するものではない。繰返しheaderは後続の構造/paintでArtifactへ接続する必要がある。正式元全巻exportと元の全巻/独立/管理host/人手受入も引き続き未完である。

§14.124の最終接続52 tests成功（2.97秒、`/private/tmp/typaxis-book-v2-placement-tests-final.log`）。コマンドは§14.120と同じoriginal Harano環境付きfeature CLI。選択sequenceそのものの借用、空ページ/空表に架空のfragmentを生成しないこと、実cellのページ内座標と一度限りのsemantic消費、各繰返しheaderのcell flag、別searchでの配置拒否、実list/footnote番号のfragment対応・baseline・一度限りの配置を検証した。実SVG数式3個のviewport寸法、native mathの実baseline/寸法、通常PNG/JPEG/SVGの実viewportも最終ページgeometryで確認した。原文/元数式geometryを変更して成功させたものではない。ページ選択と配置を通算するrecord/work上限ちょうど/1不足も検証した。

最終のdefault CLI回帰は`mixed`23件（4.42秒、`/private/tmp/typaxis-book-v2-placement-legacy-mixed.log`）、`footnote`86件（16.05秒、`/private/tmp/typaxis-book-v2-placement-legacy-footnote.log`）、`table`49件（15.91秒、`/private/tmp/typaxis-book-v2-placement-legacy-table.log`）成功。pagination feature回帰92件（0.20秒、`/private/tmp/typaxis-book-v2-placement-pagination.log`）成功。manifest/locked/target-dirは§14.120と同じ。feature CLI check・最終ビルドの警告/コンパイルエラー検査・`git diff --check`成功。全プロセスは正常終了した。これらは最終PDF/UAや元全巻受入の証拠ではなく、元の完了条件は維持する。

### 14.125 book-2の実測flow上のページ選択・配置の安定確認

`BookV2BodyMixedStablePages`を追加した。独立したページ選択を実際に繰り返し、両方を配置してから、ページ境界・強制改ページ・実source cursor/table ordinal・表fragment・脚注の需要/継続/候補・separator・本文/cell/marker座標を比較する。各分岐の発行IDは別でも、比較する定義の状態・最初の参照owner・継続位置は一致が必要である。実配置の一致が観測されるまで安定結果を返さない。

反復の制御は旧mixed pageと共有するprivate `StableSearch` kernelへ移し、record配列と定義需要の値比較も共通化した。元のsearchを使い続けるため、先行準備・全候補・各パスの選択/配置・比較のrecord/workを戻さない。宣言`max_layout_passes`と呼び出し側の残りパス数の小さい方を使い、二回の選択に足りなければ即時に`PagePassLimit`とする。上限まで異なるgeometryが続く場合も、最後の候補を安定結果として返さない。

これは不変の実測flowに対するページ安定確認である。ページ参照文字列の再生成、行の再shapingとページをまたぐ全体収束、最終math terminal・構造/PDF・公開gateは別の接続を必要とする。旧receiptへの変換はなく、元の全巻受入条件は維持する。

式番号のページ配置を追加する前の接続54 tests成功（2.70秒、`/private/tmp/typaxis-book-v2-stability-tests.log`）。original Haranoを含むfeature CLIコマンドは§14.120と同じ。空表・長い表・脚注継続を含む実ページの反復と一度限りの内容/番号配置、安定結果の実sequence owner、別search拒否、残りパス/宣言パスの不足、配置まで通算するrecord/work上限ちょうどと1不足を検証した。pagination feature93 tests成功（0.17秒、`/private/tmp/typaxis-book-v2-stability-pagination.log`）。追加kernel試験ではgeometryが各回変化する入力を両方のパス上限で拒否し、三回目で初めて一致する場合には三回分の消費を保持する。

### 14.126 book-2の式番号の実ページ配置と繰返し見出し

`BookV2BodyPlacedEquationNumber`を追加し、実placed pageが式番号のgeometryと繰返しheader flagを保持するようにした。番号のfragment indexは脚注を含む全ページfragment列を指す。元のblock・実番号shapeのsource owner/寸法/fingerprintを照合し、実parent frameの右端・数式viewportとのminimum gap・番号のtop offsetを用いて座標を算出する。番号を数式のreplacement textへ連結しない。

位置計算は旧番号処理と同じprivate関数を使う。book-2のshape照合とgeometryの保持は専用page ownerの下で行い、旧math ledgerを発行しない。繰返し見出しの番号も実表示に必要な一つのコピーとして配置し、元source番号と区別するflagを保持する。後続の構造/paintではそのコピーをArtifactとして扱う必要がある。番号検索・保持と比較を既存のwork/record予算へ加算し、§14.125の安定比較にも式番号を含める。

§14.125–14.126の最終接続55 tests成功（4.64秒、`/private/tmp/typaxis-book-v2-stability-number-final.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。実SVG数式のある表見出しを複数ページへ配置し、各式番号の親fragment・元shape fingerprint・寸法・右端・top offset・minimum gap・繰返しflagを確認した。元の意味上の番号は一つのままである。二回の実選択/配置と最終配置・番号保持を合わせたrecord/work上限ちょうどと1不足も検証した。

原ノ味の現物を読む既存CFF試験も、最終行と実block準備、ページ安定確認、式番号の実配置まで延長した。原ノ味SHA-256と実number shapeの対応、元parent/number ownerと配置幅を確認した。これは原ノ味全巻PDFやsubset後の描画/抽出を証明するものではなく、その受入は引き続き必要である。

最終default CLI回帰は`mixed`23件（1.35秒、`/private/tmp/typaxis-book-v2-stability-legacy-mixed.log`）、`footnote`86件（14.70秒、`/private/tmp/typaxis-book-v2-stability-legacy-footnote.log`）、`table`49件（14.59秒、`/private/tmp/typaxis-book-v2-stability-legacy-table.log`）成功。manifest/locked/target-dirは上記と同じ。feature CLI checkも成功。元の全体収束・math terminal/構造/PDF・正式全巻export・公開/全巻/管理host/独立/人手受入条件は変更していない。

最終pagination feature回帰93件成功（0.29秒、`/private/tmp/typaxis-book-v2-stability-number-pagination-final.log`）。コマンドは`cargo test --manifest-path workspace/Cargo.toml -p typaxis-pagination --features book-v2-staging --locked --target-dir /private/tmp/typaxis-vmb-book-build`。各最終試験ビルドのcompiler warning/errorはなく、`git diff --check`も成功。全プロセスは正常終了した。

### 14.127 book-2の安定配置と実原文・数式の消費照合

`BookV2BodySourceClosure`は実`BookV2BodyMixedStablePages`と、その同じsequenceを借用する実placed geometry、元の`BookV2PreparedBodyFlow`を保持する。同じ内容を再選択した別sequenceとの組合せや別searchを拒否する。本文と定義の局所item indexを元のglobal leafへ対応付け、owner・source種別/index・page indexを照合する。本文の全paintable leafと、参照され完了した脚注定義の全paintable leafが意味上ちょうど一回配置されたことを確認する。強制改ページはpaintable leafとして数えない。

表見出しの繰返しは表示用コピーとして別に数え、元の意味上の配置を持たないコピーを拒否する。未参照脚注定義は配置数へ足さず、`unreferenced_definitions`として明示的に保持する。これは§14.33の局所需要処理に従った照合であり、未参照定義を含む入力を公開tagged profileへ通す許可ではない。原文の意味構造やPDFのadmission判断を変更しない。

各実paragraph line内のvectorは、元bindingのfingerprint・source span・placementと選択されたatomic itemを照合し、元metrics・実pen・line baselineから共通geometry式でviewportを再確認する。native inline mathは同じ専用計算receiptへの参照、元source span、実line baselineを要求する。block vector/native mathは元binding/計算receiptと実block、viewport寸法・offset・baselineを確認する。旧native math receiptをbook-2のinline結果として受理しない。数式と通常vector figureを区別し、意味上の数式数と見出しコピーの数式数を別に保持する。式番号は借用するpage geometry上の独立した配置のままとする。

消費bitmapの保持、全leaf/inlineの走査、binding/計算receiptの検索を元の累積record/work予算へ計上する。bitmapの解放で予算を戻さず、先行した選択・全安定パス・配置と通算する。このownerは選択済み原文・数式と物理geometryの接続を証明する段階であり、dynamic label/line/page全体収束、最終math terminal spool、構造・Artifact paint・PDF/UAの証拠ではない。入れ子表/定義内表、残るauthored kinds、正式元全巻export、元の原ノ味/TrueType全巻、公開・管理host・独立・人手受入は引き続き必要である。

最終接続57 tests成功（3.09秒、`/private/tmp/typaxis-book-v2-source-math-final.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。本文/定義の一度限りの消費、強制末尾空ページ、空表/長い表/脚注継続、同一内容の別stable sequenceと別searchの拒否、累積record/work上限ちょうどと1不足を検証した。実SVG inline/block、実native inline/display、原ノ味の現物を使う式番号付き入力、通常PNG/JPEG/SVG、通常vector figureを区別し、繰返し見出しの13個の元数式とコピー、未参照の番号付き定義数式の未配置を確認した。元のsourceや数式geometryは変更していない。中間試験で通常vector figureを数式数へ含めた追加assertionが一件失敗したため、fixtureの原文を照合し、本文/定義の実数式二個を要求する正確なassertionへ修正した。

pagination feature回帰94 tests成功（0.21秒、`/private/tmp/typaxis-book-v2-source-math-pagination.log`）。コマンドは同じmanifest/locked/target-dirで`cargo test -p typaxis-pagination --features book-v2-staging`。追加の消費状態試験は元の配置欠落、二重semantic消費、未選択sourceのコピーを拒否する。feature pagination check（`/private/tmp/typaxis-book-v2-source-math-check.log`）と最終ビルドのwarning/error検査も成功。今回の変更はbook-2 feature内の新しい照合段階と接続試験であり、旧形式の実装経路は変更していない。全プロセスは正常終了した。

### 14.128 book-2の実数式配置記録と累積terminal spool

`BookV2BodyMathTerminals`は§14.127の実source closureを所有し、各inline/display数式の`BookV2BodyMathTerminal`を保持する。各記録は元の`BookV2BoundVector`または`BookV2MathReceipt`への参照、実page/flattened fragment、本文/定義内item、inline位置、table cell owner、繰返しheader flagを持つ。通常のvector figureやPNG/JPEG/SVG figureを数式terminalへ変換しない。意味上の数式数とコピー数は実source closureと完全一致を要求する。未参照定義は数式terminalを発行せず、その定義数を元closureと記録headerへ残す。

inlineでは元の行内pen・baseline・viewportへ実fragmentの原点を加える。vector blockは実viewportと元metricsの符号付きorigin_xからpenを復元し、native displayは元blockのleftと実viewportからglyph/ruleの原点を求める。nativeは元計算receiptと原点/baselineを保持し、正の面積を持つ架空のviewportを作らない。原文TeX、代替テキスト、font/resource identityや実計算結果は借用元に保持される。式番号はpage geometry上の独立した記録を参照し、数式のreplacement textへ加えない。

private algorithm `typaxis.book-2-body-math-terminals/1`で固定長のbig-endian binaryを生成する。headerはalgorithm/limits/lines/blocks/table measurements/vector bindings/native computationsの7 fingerprintと、page数・意味上/繰返し数式数・未参照定義数・配置番号数の5個のu64（計264 bytes）。数式記録は126 bytes、独立した番号記録は81 bytes。存在しないoptional値のpayloadはzeroとし、存在flagで実際のzero値と区別する。記録順は実page/fragment/inline順で、番号はその後に実page内順で並ぶ。search/branch IDや処理回数・先行spool量を出力へ混ぜず、同じ実配置は同じbytes/hashとなる。このprivate encodingは公開schemaや旧形式のJCS receiptを追加・置換するものではない。

生成前に数式/番号数から必要なspoolとrecord数をchecked arithmeticで求め、割当て前に上限を検査する。先行caller spoolを受け取り、実native計算のspoolを必須の下限として扱う。同じsearchの再生成も累積し、破棄した記録の容量を返却しない。全fragment/inline/lookup/encodingと64-byte単位のhash処理を先行した選択・安定確認・source照合のworkと通算する。異なるsearch/limitsのclosureを発行元として受け付けない。

この段階で実数式配置とその保存記録が接続された。後続のdisplay-listはこの専用ownerから実vector/native paintを生成し、構造側は繰返しコピーをArtifactとして扱う必要がある。dynamic label/line/page全体収束、構造・PDF/UA、再帰table/定義table、残るauthored kinds、正式元全巻export、原ノ味/TrueType全巻と公開・管理host・独立・人手受入は引き続き未完である。

最終接続58 tests成功（2.34秒、`/private/tmp/typaxis-book-v2-math-terminals-final.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。固定offsetでbytesを読み戻す試験側の検査は、各記録を実source/line/fragmentの座標・所有者・本文/定義/cell/コピーと照合する。元原ノ味、実native inline/display、実SVG inline/block、数式から始まる脚注、繰返し番号、未参照の番号付き定義、数式を含まない通常画像・空ページも接続した。

符号付きorigin_xの別fixtureで、inline/blockの元penを保存し、変更に応じて実記録hashが変わることを確認した。元の全巻packageや原ノ味ファイルを変更した試験ではない。同じ実配置を別search/先行record量で作った場合と、同じsearchで二度生成した場合はbytes/hashが一致し、二度目のspoolは一度目の容量を返却せず加算する。record/work/spoolの上限ちょうどと1不足、別searchと異なるlimitsの拒否、native計算spoolの下限を検証した。

pagination feature回帰94 tests成功（0.15秒、`/private/tmp/typaxis-book-v2-math-terminals-pagination.log`）。コマンドは同じmanifest/locked/target-dirの`cargo test -p typaxis-pagination --features book-v2-staging`。feature pagination check成功（`/private/tmp/typaxis-book-v2-math-terminals-check-final.log`）。最終試験のcompiler warning/errorはない。これらは専用terminal接続の証拠であり、PDF描画や全巻・公開受入の代わりにはならない。

既定featureのCLI checkも成功（2分03秒、`/private/tmp/typaxis-book-v2-math-terminals-default-check.log`）。コマンドは同じmanifest/locked/target-dirで`cargo check -p typaxis-cli --bin typaxis`。旧形式の実装経路や公開profile識別子を変更せず、新しい専用型はbook-2 feature内に閉じている。全プロセスは正常終了し、最終`git diff --check`も成功した。

### 14.129 book-2の数式terminalから実vector/native描画への接続

`BookV2MathDisplayBuilder`は実`BookV2BodyMathTerminals`と受入済みresource-set /3 ledgerを借用し、元flow/limits・実shaping owner・vector bindingsを照合する。内容hashが一致する別ledgerも元shapingの所有者ではないため拒否する。build結果の`BookV2MathDisplay`は同じterminal/ledgerを保持し、各`BookV2MathDraw`から実terminalへ戻れる。本文/定義/cell/繰返しheaderの役割を引き継ぎ、元sourceを通常画像や旧paint receiptへ置換しない。

SVGは実admitted imageとbindingのlogical ID・元hash・media・parser/IR identityを確認し、元のscale・viewport・currentColorから実変換行列と`VectorContentKey`を持つdrawを生成する。Form内容の共有に使うkeyと、各論理配置の元source identityを区別する。resource照合と行列生成は既存のprivate処理を共有し、旧形式の検査も維持する。

native数式は元font face/hash/indexと実計算receiptを用い、glyphのoriginal GID・Unicode・logical ordinal・font size・元の順序を保持する。各glyph/罫線へ実terminalのorigin/baselineを加え、元bboxから物理boundsを求める。旧形式と共有する`project_native_paints`はこの座標処理のみを行い、両方の入口がそれぞれの元source/font受入を保持する。book-2の描画結果から旧native receiptを発行する経路はない。

builderは先行terminalとcallerのrecord/workを引き継ぎ、全draw/native paintの保持数を割当て前に検査する。成功・破棄・失敗したbuildの処理量を同じbuilderに残し、再試行で予算を戻さない。生成した実draw・vector key/viewport/scale/color・native bounds/glyph/ruleを、`typaxis.book-2-math-display/1`のfingerprintへ反映する。ハッシュ入力は最大136 bytesの固定stack領域で扱い、その64-byte単位の処理量も加算する。native font sizeを含むLengthは全64-bitを保持する。先行workや再生成回数をdraw fingerprintへ混ぜず、同じ実描画は同じhashとなる。

ここで接続したのは数式の実描画命令である。本文glyph・独立した式番号・list/footnote番号・通常画像を含む全体display、Artifact/Formula構造、font plan/subsetとPDF/UAへの接続は後続である。dynamic label/line/page全体収束、再帰table/定義table、残るauthored kinds、正式元全巻export、原ノ味/TrueType全巻と公開・管理host・独立・人手受入の条件は維持する。

最終接続60 tests成功（2.39秒、`/private/tmp/typaxis-book-v2-math-display-final-2.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。実SVGのcontent key/scale/RGB/行列、実nativeのbbox・original GID/Unicode/ordinal/font size・glyph/罫線座標と順序を、元terminal/計算結果に照合した。元原ノ味を含む接続、脚注内数式、符号付きvector origin、未参照定義、数式のない通常画像も検証した。

inline/displayの分数と、native分数を含む繰返しtable headerのfixtureを追加した。table入力では13個の元数式に非重複の実source spanを与え、headerの物理コピーだけがその同じ元数式を参照する。最初の試験で本文にもnative math source spanを複製した不正なfixtureが`InvalidSourceSpan`となったため、原文を13個の実出現として生成し直した。元入力検証を緩めたものではなく、元全巻packageやフォントも変更していない。修正後は元の意味上の13数式とページごとのheaderコピーを実描画まで照合できた。

builderの二度の生成・異なる先行record/work量で同一draw hashになること、先行/破棄/失敗分が累積すること、record/workの上限ちょうどと1不足、失敗後の再試行でもworkが復活しないことを検証した。同一画像hashを持つ別ledgerと異なるlimitsを拒否する。hash計算分のworkも含む最終条件で成功した。

旧形式のnative CLI回帰21 tests成功（1.57秒、`/private/tmp/typaxis-book-v2-math-display-legacy-native.log`）。コマンドは同じmanifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis native`。display-list feature回帰57 tests成功（0.42秒）と発行権限のcompile-fail doctest 1件成功（5.98秒、`/private/tmp/typaxis-book-v2-math-display-regression.log`）。コマンドは`cargo test -p typaxis-display-list --features book-v2-staging`。feature CLI checkも成功（`/private/tmp/typaxis-book-v2-math-display-check.log`）。最終ビルドにcompiler warning/errorはなく、全プロセスは正常終了した。最終`git diff --check`成功。これらの数式描画接続を全体PDF/UAや元全巻受入の証拠として扱わない。

### 14.130 book-2の実本文clusterからページ文字描画への接続

`BookV2MathDisplayBuilder::build_text`は、同じ実terminal/source closure・resource-set /3 ledgerから`BookV2TextDisplay`を生成する。数式と本文の生成が同じbuilderの累積record/workを使用し、生成順序や再生成で先行分を戻さない。別ledgerやlimitsの拒否は共通のconstructorが維持する。結果は元terminalと受入ledgerを借用し、各`BookV2TextDraw`は選択済みの実`ProductionPlacedTextCluster`と実フォント計測値を参照する。旧形式のtext drawやpaint許可へ変換しない。

原文のcluster単位・元glyph ID・順序・UTF-8を保ち、実行内glyph座標へ物理fragmentの原点を加える。複数glyphのclusterをUnicodeから再構成しない。フォントface/hash/indexを実admitted fontへ照合する。logical boundsには実pen/advanceとascender/descender・page baselineを使用し、advanceまたは高さがzeroなら正の面積のboxを作らない。この座標計算のみをprivate `project_cluster`へ抽出し、旧本文描画と共有した。旧経路のsource/font照合とfingerprintは維持する。

元paragraph lineからのdrawにpage・flattened fragment・本文/定義内item・inline位置・cell owner・繰返しheader flagを保持する。脚注や表の表示用コピーでも元clusterへの参照を失わない。生成文字は同じstore-issued provenanceを保持し、元1.5 wireのparsed buffer数とgenerated buffer IDをchecked arithmeticで結合してdisplay namespaceを割り当てる。元のparsed spanとの衝突を避け、番号の文字列を原文や数式replacementへ連結しない。独立した式番号、list/footnote markerの描画はこの本文cluster処理へ混ぜず、後続の専用接続を必要とする。

全ページ/fragment/inlineを事前走査してdraw/glyph数をchecked arithmeticで求め、配列を割り当てる前に全recordを累積予算へ計上する。後でwork上限や描画処理が失敗した場合にも、未到達のdraw用に確保した容量を含めて保持する。共有geometry関数には計上済みの各cluster分だけの残量を渡す。数式側でも同じ割当て規則へ修正した。従来の数式処理は全量を事前検査していたが、配列確保後の各draw到達時に計上していたため、途中失敗で未到達slotが消費に含まれなかった。正常時の描画内容・fingerprint・消費量は変わらない。

private algorithm `typaxis.book-2-text-display/1`で、実source/ledger fingerprintと各drawのowner・位置/役割・run/cluster index・source namespace/span・元UTF-8・font face/hash/index/size・logical bounds・実glyph ID/座標をhashへ反映する。固定stack領域を用い、長いUTF-8は長さを先に記録して104-byte以下のchunkで処理する。64-byte単位のhash workと全走査/実glyph投影のworkを累積する。処理順を変えたmath/text生成や異なる先行予算値でも、同じ実本文描画は同じhashとなる。

ここで接続したのは実本文文字のdrawであり、inline anchor/navigation、独立番号/marker、通常figureを含む全display、繰返しコピーのArtifact/本文構造、font plan/subset・PDF/UAへの接続は引き続き必要である。dynamic label/line/page全体収束、再帰table/定義table、残るauthored kinds、正式元全巻export、原ノ味/TrueType全巻と公開・管理host・独立・人手受入の完了条件は維持する。

最終接続61 tests成功（2.55秒、`/private/tmp/typaxis-book-v2-text-display-final-3.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。試験側は実page/fragment/inline列から各元clusterを探し、pointer identity・元UTF-8とparsed/generated span・実フォントhash/index・glyph ID/順序/座標・logical boundsを独立に照合する。本文・脚注・繰返しtable headerと生成参照文字、実SVG/native数式の前後、通常figureに伴う本文を検証した。繰返し文字は元clusterと同じ原文/provenanceを持ち、意味上のsourceを二重消費しない。

元の原ノ味を用いる日本語IVS試験も、実行の再shaping・ページ安定確認・source/terminal・文字drawまで延長した。日本語と異体字セレクタを含む原文をdraw列から復元し、`日`+U+E0100が一つのclusterのままで、元フォントの実glyph IDとSHA-256を保つことを確認した。小さい試験fixtureのpage寸法を実文字が収まる値へ設定したもので、正式元全巻package・原文・フォントを書き換えたものではない。未収録variation sequenceの拒否も引き続き成功した。

本文/数式を組み合わせた累積record/workの上限ちょうどと1不足、異なる先行work、生成順序の変更、本文再生成で同じhashになることを確認した。work上限を事前走査の直後に設定した試験では、最初のdrawを生成する前に失敗しても全予約recordが計上されたままとなる。本文再試行はworkを復活させず、未到達slotを返却しない。数式側の同じ早期失敗条件でも全draw/native paint容量の保持を検証した。

既定featureの文字関連CLI回帰29 tests成功（2.09秒、`/private/tmp/typaxis-book-v2-text-display-legacy-text-final.log`）。コマンドは同じmanifest/locked/target-dirの`cargo test -p typaxis-cli --bin typaxis text`。最初はincludeされたファイル名をfilterに指定して0件となったため、その結果を回帰の証拠にせず実test名の`text`で実行した。実本文/PDF文字の座標・font・source、脚注文字と番号/数式/区切り線、構造の関連試験が成功した。

display-list feature回帰57 tests成功（0.44秒）と発行権限のcompile-fail doctest 1件成功（4.38秒、`/private/tmp/typaxis-book-v2-text-display-regression.log`）。コマンドは同じmanifest/locked/target-dirの`cargo test -p typaxis-display-list --features book-v2-staging`。最終接続/回帰ビルドにcompiler warning/errorはなく、全プロセスが正常終了した。最終`git diff --check`成功。これらは本文draw接続と共通geometry回帰の証拠であり、全体PDF/UAや元全巻・公開受入は未完である。

### 14.131 book-2の独立した式番号から実文字描画への接続

`BookV2MathDisplayBuilder::build_equation_numbers`は実page geometryの独立した式番号を読み、元の`BookV2EquationNumberShape`とparent/number owner・shape fingerprint・幅/高さを照合する。`BookV2EquationNumberDisplay`は実terminal/source closureと受入ledgerを借用し、各`BookV2EquationNumberDraw`が実`BookV2BodyPlacedEquationNumber`と元shapeを参照する。flattened fragment・page・繰返しheaderの役割は元placementに保持され、原文や数式replacementへ番号を加えない。未参照定義の未配置番号についてdrawを生成しない。

番号フォントの水平計測をprivate `equation_number_font_metrics`へ共有化した。旧形式と新しい`book_v2_equation_number_font`の各入口が、それぞれの実admitted fontのface/hash/indexを照合してから、元SFNTのhheaとunits-per-em・実font sizeを使用する。原ノ味の元CFF /2も同じhhea計測へ接続し、架空のfont metricsや旧shape receiptを発行しない。

元の番号座標計算はprivate `project_number`へ共有化した。実line-heightとascender/descenderから符号付きhalf-leadingを求めてbaselineを配置する。runのadvanceを計測し、UAX #9 L2で視覚的なrun原点を求める一方、出力clusterは元source順に保持する。各glyphの元GID・offset/advanceを使い、結合文字を含むclusterを分割・再構成しない。`ProductionEquationNumberCluster`はrun/cluster index、元parsed spanとUTF-8、実glyph座標、logical boundsを保持する共通geometry値であり、旧形式のpaint許可ではない。旧番号経路は同じgeometryから自分の実shape許可を保持したdrawを発行する。

本文/数式と同じbuilderを使用し、番号draw・cluster/glyph保持・runの幅/順序/原点配列・一時glyph位置配列をすべて割当て前に累積recordへ計上する。検索・事前走査・実glyph計算・双方向文字の各反転/走査・hash処理も同じworkへ加算する。失敗や再生成によって一時配列や未到達slotの容量を戻さない。`typaxis.book-2-number-display/1`は元terminal/ledger/shape、配置owner/page/fragment/繰返し役割、font instance/実metrics、各元cluster/span/UTF-8と実glyph座標を固定stack領域でhashへ反映し、先行予算や生成順序を描画identityへ混ぜない。

独立した式番号の実drawが接続された。list/footnote marker、通常figure、inline anchor/navigationを含む全display、本文/Formula/Artifact構造とfont plan/subset・PDF/UAは後続である。dynamic label/line/page全体収束、再帰table/定義table、残るauthored kinds、正式元全巻export、原ノ味/TrueType全巻と公開・管理host・独立・人手受入の完了条件は維持する。

双方向番号の追加試験では、元Arial Unicodeのヘブライ文字・結合記号・数字を含む入力がshaping後のsource coverage照合で拒否された。原因はbook-2が旧receiptのbidi level 0/1制限を引き継いでいたことだった。共通のcoverage計算へ上限を渡す形にし、旧形式の入口は1を維持、book-2は共通`BidiLevel`型の`MAX_BIDI_LEVEL`（125）を使用する。run ID・原文の連続性・非空cluster・glyph範囲の照合を省略する変更ではない。旧公開schema/receiptを新しいlevelへ広げず、1.5の専用shapeだけに適用する。

最終接続62 tests成功（4.94秒、`/private/tmp/typaxis-book-v2-number-display-final-2.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。同じ原文/shape/placementへの参照、実font identityと寸法、各run/clusterの元parsed span/UTF-8、glyph ID・offset/advance・logical boundsを照合した。試験側はrun対のembedding levelの最小値の偶奇から視覚順を求め、実装側の段階的な反転処理とは別の計算で各glyphのx/yを検査する。

元のArial Unicode.ttf（SHA-256 `876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec`）を読む追加試験は、ヘブライ文字・結合記号U+05B0・数字を混在させた番号`(אבְ 12 ג)`を使う。複数のbidi levelと複数glyphを持つclusterを実際に要求し、短いline-heightによる負のhalf-leading、元UTF-8の完全な復元、番号の文字を本文へ含めないことを確認した。元の原ノ味を用いる独立番号、実SVG数式の繰返しtable header、未参照定義の未配置番号も共通の描画検査を通る。正式元全巻packageや元フォントを書き換えた試験ではない。

数式・本文・番号の生成を合算したrecord/work上限ちょうどと1不足、異なる先行work、生成順序の変更、番号の再生成、失敗/再試行後の累積消費を検証した。実描画のhashは同じで、失敗時にも確保した全配列と未到達slotの消費を保持する。

既定featureの番号関連CLI回帰8 tests成功（1.56秒、`/private/tmp/typaxis-book-v2-number-display-legacy.log`）。コマンドは同じmanifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis number`。実式番号の原子配置/描画、CID共有と抽出文字、container gapと強制改ページ、繰返しheader、脚注内番号と周辺描画が成功した。display-list feature回帰57 tests成功（0.55秒）と発行権限のcompile-fail doctest 1件成功（4.42秒、`/private/tmp/typaxis-book-v2-number-display-regression.log`）。コマンドは同じmanifest/locked/target-dirの`cargo test -p typaxis-display-list --features book-v2-staging`。

追加のshaping coverage試験1件成功（0.00秒、`/private/tmp/typaxis-book-v2-number-display-coverage.log`）。コマンドは同じmanifest/locked/target-dirで`cargo test -p typaxis-shaping --features book-v2-staging successor_embedding_levels`。level 2の同じrunを旧receiptの入口では拒否しbook-2の上限では受け付けること、glyph範囲外と不正なrun IDを引き続き拒否すること、level 0の旧受付を維持することを検証した。feature CLI check成功（`/private/tmp/typaxis-book-v2-number-display-check.log`）。最終接続/回帰/coverageビルドにcompiler warning/errorはなく、全プロセスは正常終了した。最終`git diff --check`成功。全体PDF/UA・元全巻・公開受入は未完のままである。

### 14.132 book-2のリスト・脚注markerと区切り線の実描画

`BookV2MathDisplayBuilder::build_markers`は実page geometryのlist/footnote markerを読み、元の`ProductionListMarkerShape`または`ProductionFootnoteMarkerShape`へ接続する。`BookV2MarkerDisplay`は同じ専用terminal/source closureとresource-set /3 ledgerを借用し、各`BookV2MarkerDraw`は実marker shape・実placement・配置先fragmentへの参照を保持する。page内の局所fragment indexと全体のflattened indexを区別し、定義内item・cell owner・繰返しheaderの役割を引き継ぐ。空の配置や未参照定義から架空のmarkerを生成しない。

markerは実fragment順に並べ、同じfragmentに脚注定義番号とlist番号がある場合は定義番号を先にする。元の二つの配置列をiteratorで併合し、並び替え用の文書サイズ配列を追加しない。shapeの元owner/index、実font face/hash/index、計測advanceと配置幅、本文と定義のfragment対応を照合する。文字の出所は元store-issued `GeneratedProvenance`のままであり、旧receiptや本文のparsed textへ変換しない。

private `project_marker`に元glyphのページ座標計算を共有化した。入力runと完全な生成provenance、元owner/keyを照合し、各clusterが同じ生成bufferの連続したsubspanであることとglyph範囲の連続性を確認する。UTF-8は元全体spanに対する相対範囲から取得し、元GID・offset/advanceを保持する。最後に原文/glyphを全量消費したこと、総advanceが実marker幅と一致すること、累積Y advanceがzeroであることを要求する。ゼロ幅clusterに正の幅を作らない。旧形式も同じgeometry処理を呼び、自分のsource/font照合とdraw許可を保持する。

`ProductionGeneratedMarkerCluster`は実cluster index、generated provenance、parsed buffer群の後ろに割り当てたdisplay span、元UTF-8、実glyph座標・logical boundsを保持する共通geometry値である。book-2は元1.5 wireのparsed buffer数からdisplay namespaceをchecked arithmeticで計算する。本文中の生成脚注参照と脚注領域の定義番号は、同じ生成bufferを参照できても、実際の別配置としてそれぞれ保持する。

`BookV2FootnoteSeparatorDraw`は元pageの実separator inkと、最初の脚注fragmentのflattened indexを保持する。全体displayで脚注本文より前に挿入できる位置を渡し、架空の本文glyphやsource nodeを作らない。最終のArtifact指定は構造/paint側への接続を必要とする。

本文・数式・独立式番号と同じbuilderで、全marker draw/cluster/glyphとseparatorの容量を割当て前に累積recordへ計上する。事前走査・実glyph投影・separator位置検索・固定stackのhash処理を同じworkへ加算し、成功・破棄・途中失敗・再試行で容量を返却しない。`typaxis.book-2-marker-display/1`のhashは元terminal/ledger/marker identity、役割と実配置、各生成cluster/span/UTF-8・glyph座標、separator inkと挿入先を反映する。異なる先行予算や生成順序は実描画identityを変えない。

リスト・脚注markerと区切り線の実drawが接続された。通常figureとinline anchor/navigationを含む全display、本文/Formula/Artifact構造とfont plan/subset・PDF/UAは後続である。dynamic label/line/page全体収束、再帰table/定義table、残るauthored kinds、正式元全巻export、原ノ味/TrueType全巻と公開・管理host・独立・人手受入の完了条件は維持する。

最終接続65 tests成功（4.54秒、`/private/tmp/typaxis-book-v2-marker-display-final-3.log`）。コマンドは`TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。試験側は元pageの別々のmarker列から期待する順序を求め、各実placement/shape/fragmentへの参照、生成storeの元文字/provenance、font identity、glyph ID・座標・cluster幅、separator inkと最初の脚注fragmentを独立に照合する。

追加の12脚注fixtureでは、名前の辞書順に依存しない元定義順の`1`–`12`、本文listの`1.`–`12.`、脚注内listの`98.`を一度ずつ描画する（計25 marker）。同じfragmentの脚注定義番号がlist番号より先に現れることも確認した。別のtable fixtureでは、見出しlistの`10.`が元の意味上の一個と複数の表示用コピーになり、同じ元provenance/fingerprintを保持することを確認した。

元の原ノ味を用いる追加fixtureでは、段落・通常vector figure・独立番号付き数式から始まる三つのunordered list itemに、それぞれ元glyphによる`•`を配置した。三個のmarkerの元font hashと非zero GID、通常figure/数式の実fragment対応を確認した。元フォントや正式元全巻packageを書き換えた試験ではない。これまでの原ノ味IVS、元Arial Unicodeの双方向式番号、native/SVG数式、脚注・表・通常画像の接続も引き続き成功した。

本文・数式・式番号・markerを合算するrecord/workの上限ちょうどと1不足、先行work、生成順の変更、再生成hash、失敗/再試行後の消費保持を検証した。事前走査の直後をwork上限に設定した場合は最初のdraw生成前に失敗し、全marker/glyph/separator予約を保持したままで、再試行でも容量やworkが復活しない。

既定featureの脚注CLI回帰86 tests成功（9.53秒、`/private/tmp/typaxis-book-v2-marker-display-legacy-footnote.log`）、リスト回帰11 tests成功（1.48秒、`/private/tmp/typaxis-book-v2-marker-display-legacy-list.log`）。コマンドは同じmanifest/locked/target-dirでそれぞれ`cargo test -p typaxis-cli --bin typaxis footnote`と`cargo test -p typaxis-cli --bin typaxis production_list_`。実脚注PDF/区切り線Artifact・元番号/文字・ページ継続、nested list、数式/画像先頭item、生成glyph予算、独立PDF probeの関連試験が成功した。feature CLI checkも成功（`/private/tmp/typaxis-book-v2-marker-display-check.log`）。最終ビルドにcompiler warning/errorはなく、全プロセスは正常終了し、最終`git diff --check`成功。全体PDF/UA・元全巻・公開受入は未完である。

### 14.133 book-2通常図版と行内アンカーのページ座標への接続

`BookV2MathDisplayBuilder::build_images`を追加し、実際に選択・配置された通常Figure（PNG/JPEG/SVG）、`InlineVector`、`VectorFigure`を`BookV2ImageDisplay`へ接続した。各drawは元の`ProductionPreparedFigure`または`BookV2BoundVector`、実際の`AdmittedImage`とページfragmentを借用し、flattened fragment番号・行内item番号・脚注定義・table cell/repeated-header役割を保持する。`MathVector`/`MathVectorBlock`は既存の数式drawに残り、通常画像として重複しない。captionは元本文のtext drawのままである。

通常Figureは元画像hash、準備時の寸法と実viewport、raster pixel寸法または実SVG content keyを検証する。通常SVGの黒currentColorと、precomposed vectorの元scale/currentColor/viewportを保持し、共有のresource key検証と`placement_matrix`でページ変換を作る。行内viewportは元の選択済みgeometryとfragmentの原点から計算する。元画像bytes/代替テキストを置換せず、後続の構造・resource/PDF段階が実ソースを参照できる。旧profile用の描画許可を発行しない。

`build_anchors`は`BookV2AnchorDisplay`を作り、元の非描画anchorと実fragmentを借用して、選択済み行のx/baselineをページ座標へ変換する。本文・脚注・table headerの再掲役割を保持し、実fragment baselineとの一致を検証する。段落anchorの二分探索範囲だけを各行で走査し、章全体のanchorを各行で繰り返し全走査しない。行を持たない段落の元anchorは`unpositioned`に明示的に保持し、架空の座標を割り当てない。未参照脚注の配置済み行内anchorは元の未描画source flowに残る。anchor自体はglyph/画像/MCIDを生成しない。空段落のdestination解決や最終リンク許可は後続navigationの未完事項である。

両componentは描画を作る前に全保持recordを予約し、実ページの走査、元bindingの検索、画像確認、アンカー検索と固定長stack chunkによるhash処理を既存builderの累積workへ加算する。`typaxis.book-2-image-display/1`と`typaxis.book-2-anchor-display/1`は元のsource、実owner/役割/fragment・座標・画像identity/代替テキスト・vector geometryまたは元anchor span/gapを結び、会計counterやcomponent生成順に依存しない。失敗・再試行でも予約容量とworkを戻さない。

図版・アンカーのcomponent接続までであり、描画全体の順序付き統合、navigation/構造/Artifact/font-plan/subset/PDF、動的label/行/ページ収束、再帰table/脚注内table、残る著者kind、正式元全巻export、元原ノ味/TrueType全巻および公開・管理host性能・独立検証・人手受入は引き続き未完である。全体goalの完了は主張しない。

追加fixtureで、anchorのみの空段落は行layoutまで元anchor/position=Noneを保持するが、現在の本文flow準備が`EmptyParagraph`として拒否することを実測した。この入力はまだ`BookV2AnchorDisplay`まで到達しない。上記`unpositioned`は座標を捏造しないための保持口であり、空段落destinationを受理できたという意味ではない。空段落flow/位置解決の接続は未完事項として残し、拒否を専用回帰で固定する。

### 14.134 book-2の順序付き本文描画の統合

`BookV2MathDisplayBuilder::build_body`で、同じ実source/入場済みresourceを借用する本文text・native/SVG数式・独立式番号・list/脚注markerと区切り線・通常画像・非描画anchorを所有する`BookV2BodyDisplay`を作る。別sourceから作られたcomponentを混ぜる公開constructorは設けない。元の各drawとそのowner/role/geometryを保持したまま、`BookV2BodyPaintIndex`の列で最終的な描画順を指定する。

順序は実flattened fragmentを基準に、脚注区切り線、同fragmentの脚注/list marker、元行内item順のtext/数式/図版、独立式番号とする。marker同士は既存componentの脚注番号先行順を保持する。数式番号は数式の置換テキストへ吸収しない。anchorは非描画記録のままであり、順序列に架空paintを作らない。再掲headerの役割と元の意味上の一個は各componentに残る。

六つのstreamを固定長stack cursorでmergeし、全巻分の追加sorting bufferは作らない。実候補比較、走査、hashは既存累積workを使い、順序列全件と所有recordを確保前に予約する。各streamの消費完了、出力順の単調性、同fragment/inline位置の本文paint重複を検証する。`typaxis.book-2-body-display/1`は全component fingerprintと実際のinterleavingを結び、先行counterや再生成に依存しない。後段失敗でも先に作った全componentと順序列の会計消費を戻さない。

この段階はverified sourceに結び付く私有の順序付き描画であり、公開book-2登録やPDF構造/paint許可を発行するものではない。navigation/構造/Artifact/font-plan/subset/PDF、空段落flow/destination、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportと全ての全巻/公開/管理host/独立/人手受入は未完である。

最終統合検証（§14.133–14.134共通）は67 tests成功（4.60秒、`/private/tmp/typaxis-book-v2-body-display-final-2.log`）。コマンドは次のとおりで、元フォント/正式全巻packageは変更していない。

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' \
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  --features book-v2-staging book_v2_resources --locked \
  --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored
```

実配置から独立に期待列を作り、元Figure/binding/画像/fragmentのpointer、元bytes/hash/代替テキスト、raster寸法、SVG content key/scale/RGB/matrix、行内viewport、脚注/table/repeated-header役割、元anchor/座標/非描画性を比較した。追加の複合fixtureでは本文hard break前後の四anchor、脚注anchorと通常画像、同じ元画像・元anchorを再掲するtable header、未参照脚注の非描画を検証した。元の意味上のheader画像は一個であり、コピーは別ページ/再掲役割を持つ。anchor-only空段落は別の一件で実`EmptyParagraph`拒否を確認した。初期fixtureの非連続style source_orderは正しく拒否されたため、規則を緩めず実配列順へ修正した。

全描画順は、各実fragmentでcomponentを独立に走査して元inline順を構成するtest側oracleと一致した。各componentの全件が一度ずつ現れ、独立式番号は対応する数式の直後、脚注区切り線・脚注/list番号は本文より前であり、非描画anchorは順序列へ混入しない。元原ノ味のIVS/箇条書き/式番号、元Arial Unicodeの双方向式番号、native fractionのglyph/rule、SVG数式、通常PNG/JPEG/SVGとcaptionが同じ統合経路を通る。

図版/anchorの上限ちょうど・1不足、先行work、生成順変更、再生成hashと会計消費を検証した。会計観測から事前走査直後の予約境界を二分探索するoracleにより、最初の予約直後のwork失敗でも全recordが保持され、再試行が消費を復活させないことを確認した。全本文統合にもcombined record/workの上限ちょうど・1不足、先行work、再生成した順序列/hash、後段失敗/再試行時の消費保持の試験を適用した。空の数式componentも所有recordを予約するため、work枯渇後の統合再試行が追加recordを消費する場合がある。試験はこの実消費を認めつつ返却・work復活がないことを確認する。

図版/anchor追加時点のdisplay-list feature回帰57 tests（0.55秒）とowner-issuance compile-fail doctest 1件（4.85秒）が成功（`/private/tmp/typaxis-book-v2-image-anchor-display-regression.log`）。コマンドは`cargo test --manifest-path workspace/Cargo.toml -p typaxis-display-list --features book-v2-staging --locked --target-dir /private/tmp/typaxis-vmb-book-build`。統合追加後の既定feature CLI checkも成功（47.42秒、`/private/tmp/typaxis-book-v2-body-display-default-check.log`）：`cargo check --manifest-path workspace/Cargo.toml -p typaxis-cli --locked --target-dir /private/tmp/typaxis-vmb-book-build`。最終統合・回帰・既定checkにcompiler warning/errorはなく、全プロセスは終了済み。全体PDF/全巻/公開受入の完了は主張しない。

### 14.135 book-2描画由来のfont usageと共通glyph選択

`BookV2BodyDisplay::font_slot_count/font_use`で、順序付き描画の各paintから実font使用を取得する入口を追加した。`BookV2FontUse`のconstructorは非公開であり、元の本文cluster、生成list/脚注marker cluster、独立式番号cluster、native数式glyph以外の値で置換して発行できない。元font instanceと実入場ledger/face/hash/face indexを照合する。本文/番号/markerは実font-instance table、native数式は実native receiptのinstance tableを参照する。glyph ID・サイズ・UTF-8・parsed/generated display spanを保持し、native数式は元receipt/computation hash・paint index・logical ordinalとUnicode scalarを保持する。native glyphへ架空のtext spanを割り当てない。SVG/通常画像/区切り線はfont使用を持たず、native ruleのslotもfont使用を返さない。

`BookV2BodyDisplay::verify_resources`は実admitted ledgerのpointerと元flowのlimitsを検証し、後続resource段階の入口とした。`typaxis-resources`の`book-v2-staging` featureに`BookV2FontSelectionBuilder`を追加し、元の実描画を所有者として全font usageを元paint/cluster順に保持する。同じ実instance tableに属するglyphをFontInstanceId/GID順に並べ、再掲headerなどの重複を取り除いたfont別glyph範囲を作る。元TrueType/TTCとCFF /2の両方を同じ選択処理へ渡し、未使用fontやvector中の図形をfont使用として追加しない。これは元glyphの選択であり、`.notdef`/複合glyph closure、実subset生成、CID/ToUnicode/ActualText/font PDF objectの確定は後続の未完処理である。旧profileのencoder receiptやPDF paint許可へ変換しない。

usage・glyph・font配列の全recordと実型サイズに基づく保持byteを確保前に予約する。font配列は実instance table長とglyph数の小さい方を上限とし、再掲glyph数だけfont recordを過剰確保しない。glyph重複排除で確保済み容量を返却しない。元displayのrecord/work、元terminal/native computationのspoolとcallerの先行消費を下限とする累積builderである。glyph並べ替えは固定追加領域のfallible heapsortを使い、比較・swapごとにwork上限を確認し、上限到達時にその場で失敗する。単純な二乗時間の挿入や、workが尽きた後も比較を続ける標準sort callbackへ依存しない。hashも固定長stack bufferで計数する。`typaxis.book-2-font-selection/1`は元display/admission、実usage順、font instance/table/content hashと元GID選択を結び、会計counterや再生成に依存しない。

元描画からのfont使用と実glyph選択まで接続した。実subset/フォント埋込み計画、navigation/構造/Artifact/PDF、空段落flow/destination、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportおよび元全巻/公開/管理host/独立/人手受入は未完であり、全体goalは継続する。

最終統合68 tests成功（9.82秒、`/private/tmp/typaxis-book-v2-font-selection-final-4.log`）。実描画の各paint/slotから独立に期待font使用を取り出し、元glyph sliceのpointerまたはnative GID、元UTF-8/scalar/source span/native identity、実font instance table・admitted font pointer・sizeを比較した。test側BTreeMap/BTreeSetによる期待選択と、productionのfallible heapsort・重複排除によるFontInstanceId/GID順が一致した。各表示用コピーのusageを保持しながら元glyph集合を重複なく選択する。画像/rule/範囲外paint・slotに架空のfont使用が出ないことも確認した。

追加の元原ノ味+TTC fixtureは、同一の実本文でCFF /2とTrueTypeを併用し、元の二つのfont bytes/face identityを維持した二個のfont選択を確認した。元原ノ味hashは`66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717`。既存の元原ノ味IVS/箇条書き/式番号、元Arial Unicode双方向式番号、native fraction、本文/脚注/table再掲と未参照定義が同じ選択検証を通る。正式元全巻packageや元fontを書き換えたものではない。

combined record/spool/workの上限ちょうど・1不足、先行work、再生成fingerprint、後段失敗/再試行の消費保持を検証した。さらに会計観測から最初の予約直後のwork失敗境界を二分探索し、確保予定の全record/byteが保持されることを確認した。成功後・失敗後・再試行時に予算が復活しない。空の選択を再試行すると所有recordを追加消費する場合もあるため、会計の単調性を確認する。

コマンドは以下のとおり。

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' \
cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis \
  --features book-v2-staging book_v2_resources --locked \
  --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored
```

feature CLI check成功（30.47秒、`/private/tmp/typaxis-book-v2-font-selection-check.log`）、既定feature CLI check成功（54.74秒、`/private/tmp/typaxis-book-v2-font-selection-default-check.log`）。同じmanifest/locked/target-dirでそれぞれ`cargo check -p typaxis-cli --features book-v2-staging`と`cargo check -p typaxis-cli`を実行した。最終統合とcheckにcompiler warning/errorはなく、全プロセスは終了済み。元glyph選択は実装済みだが、実subset/CID/ToUnicode/フォントPDF計画と全体受入はまだ完了していない。

### 14.136 book-2 font閉包と実書き出し前の準備

`BookV2FontSelectionBuilder::prepare_font_closures`を追加した。同じ実本文displayに由来する選択だけを受け取り、先行record/spool/workを引き継ぎ、全fontの閉包をType2評価やsubset書き出しより先に準備する。`BookV2FontClosures`は実選択を借用し、各`BookV2ClosedFont`は元のselected fontを借用する。TrueType/TTCは元SFNT table/locaと実複合glyph参照を保持し、CFF /2は既存の封印された`Cff1GlyphClosureV2`を使う。どちらも`.notdef`を含む昇順の元glyph列と、元glyphから連続したsubset GIDへの対応を持つ。元TrueType/CFFを旧profileのfont receiptへ変換しない。

TrueTypeの共有`subset_truetype`を準備と書き出しへ分けた。既存の入口は同じ二段階を直ちに実行するため、成功時のfont bytes/複合glyph再配置/metrics/namingは維持する。新しい準備入口は、table map、loca、元glyph集合、探索待ち配列と元→subset対応表の確保前にrecord/byteを計数し、走査・tree操作にもworkを課す。複合glyphの子を一時Vecへ集めず、既存の検証済み歩査callbackから直接訪問する。探索待ち配列はglyph数を上限に拡張分を先に予約する。ordered treeに関するworkは16-bit key領域に対する保守的な128単位を用いる。既存の入口もこの共有kernelを使い、従来の上限所有者を変更しない。

CFF閉包ではrequested集合・source GID配列・canonical文字列の保持上限を計数して既存/2選択へ渡す。TrueTypeにも実selected CID数の上限を適用し、`.notdef`や複合部品を使用CID数へ混同しない。閉包の配列やmapへの照会・固定stackでのhashも同じ累積builderで計数する。`typaxis.book-2-font-closures/1`は元選択・元font identityと実GID対応を結び、先行会計や再生成に依存しない。同じbytes/hashを持つ別displayの選択はowner不一致として拒否する。準備失敗時にも予約済み容量と消費workを戻さない。

実本文font選択から書き出し前の閉包までを接続した段階であり、book-2からの最終font binary書き出し・CFF評価セッションの全体接続・CID/ToUnicode/ActualText/PDF font object、navigation/構造/Artifact/PDF、空段落flow/destination、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportと全ての全巻/公開/管理host/独立/人手受入は未完である。準備だけで部分埋め込み・全体goalが完了したとは扱わない。

最終統合69 tests成功（4.75秒、`/private/tmp/typaxis-book-v2-font-closures-final-2.log`）。元原ノ味+TrueType/TTC混在、IVS、元Arial双方向式番号、native数式、本文/番号/marker、脚注/table再掲を通る実選択から、元selected-font pointer、font種別、昇順/重複なし/先頭.notdef、元glyph範囲、selected glyphの全包含と連続subset番号を照合した。CFFは元selected glyphと.notdefの正確な集合、TrueTypeは複合部品を含む閉包を保持する。再準備の閉包hash一致、同じbyte/hashでも別displayのowner拒否、combined record/spool/workの上限ちょうど・1不足、先行workと失敗/再試行時の消費保持が成功した。

追加のCID容量fixtureでは、元`Result`の実使用6 glyphに対し、上限1でsubset書き出し前に`SelectedGlyphLimit`、上限6で.notdefを含む7 glyph閉包の成功を確認した。.notdefを使用CIDとして数えて誤拒否しない。元fontや正式全巻packageを改変した試験ではない。

共有subset kernel回帰4 tests成功（0.01秒、`/private/tmp/typaxis-book-v2-closure-kernel-final-2.log`、原ノ味専用の既存一件はこのコマンドではignoreのままで成功件数へ含めない）。追加した課金検証では、元glyph 3がglyph 2を参照する既存compound fixtureの閉包が`[0,2,3]`、実書き出しbytesが従来入口と一致、埋込み先glyph 2の子がglyph 1へ再配置されることを確認した。準備のrecord/byte/work上限ちょうど・1不足も確認した。同じ実出力は既存read-fontsによるmaxp/hhea/head/hmtx/loca/glyfの独立parseに通る。

既定featureのCLI text回帰29 tests成功（1.88秒、`/private/tmp/typaxis-book-v2-font-closures-legacy-text.log`）。実本文/脚注/番号のfont/text/構造・PDF probeと元anchorの既存試験を含む。最後に変更したtree workの保守係数は旧入口のno-op課金callbackの引数であり、旧byte/配置には影響しないことをコードと最終共有kernel試験で確認した。feature CLI check成功（21.87秒、`/private/tmp/typaxis-book-v2-font-closures-check.log`）。

統合コマンドは、§14.135と同じ元原ノ味・Arial Unicodeの環境変数を指定した`cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。kernel回帰は同manifest/locked/target-dirで`cargo test -p typaxis-resources --lib subset`、既定回帰は`cargo test -p typaxis-cli --bin typaxis text`。最終統合/kernel/既定回帰/checkにcompiler warning/errorはなく、全プロセスは終了済み。全体goal、book-2の実binary/フォントPDF計画・全巻/公開受入は引き続き未完である。


### 14.137 book-2の実TrueType/TTC部分埋め込み

`BookV2FontSelectionBuilder::write_truetype_font`を実閉包へ接続した。同じ本文displayの閉包とそのfont indexだけを受け取り、元closed-fontを借用する`BookV2TrueTypeSubset`を生成する。実SFNT bytes、元font単位のadvance、PDF metrics、実instance IDによるPostScript名、bytesのSHA-256と閉包に結びつく内部fingerprintを保持する。CFFをこの入口でTrueTypeへ変換せず、別display・存在しないfont index・異なるfont種別は明示的に拒否する。CID／ToUnicode／PDF receiptを生成する入口ではない。

共通`truetype_subset_writer`は元閉包からglyfの各padding、loca/hmtx、保持tableとdirectoryの実長を計数し、`max_font_subset_bytes`を出力確保前に検査する。元複合glyph参照とhmtxを先に検証し、全出力buffer・table headers・元advance map・一時nameの容量をrecord/spool/workへ課金する。glyphを別Vecへcloneせず、元の不変sliceを歩査し、出力glyfの参照番号を直接dense subset GIDへ置き換える。glyphごとの置換offset Vecも作らない。実instance名を初回の書き出しで埋め込むので、book-2でSFNT全体を再copyして名前だけを変更する必要はない。

従来のTrueType入口は同じwriterへinstance 0のplaceholder名を渡し、既存receipt段階の名前置換を維持する。compound fixtureではinstance 37で直接生成したbytesが従来の名前置換結果と一致し、元glyph 3→2は埋込glyph 2→1に変換される。read-fontsによる実SFNTの独立parse、必要なrecord/byte/workちょうど・1不足、font byte上限の1不足で出力確保がまだ0であることを確認した。

本文・native数式・番号・marker・脚注・table再掲の実選択を通じ、生成SFNTのchecksum、maxp/hheaのglyph数、元TT/TTC faceのhmtx advance/side bearing、UPM、実name table、loca/glyf長とdense対応を検証する。同一ownerでの再生成は同一bytes/metrics/fingerprintを返し、累積記録と容量は返却しない。record/spool/work上限ちょうどと1不足、先行work、途中失敗と再試行を検証した。元Harano+TTC混在fixtureのTTC側も実生成に通る。

書き出し段階の統合69 tests成功（4.79秒、`/private/tmp/typaxis-book-v2-tt-writer-final.log`）、共有subset回帰4 tests成功（`/private/tmp/typaxis-book-v2-tt-writer-kernel.log`、この段階の原ノ味専用一件はignore）、既定CLI text回帰29 tests成功（1.97秒、`/private/tmp/typaxis-book-v2-tt-writer-legacy-text.log`）。`closure_charge`の一括work課金は定数時間の検査・加算に変更し、失敗時に上限まで消費した値と事前record/spool予約を維持する。予算単位数だけ空ループを回す処理を行わない。最終CFF共通kernel変更後の再検証は次節に記録する。

### 14.138 book-2の累積CFF glyph評価

同じ`BookV2FontSelectionBuilder`が一つの`Cff1SubsetSessionV2`を保持し、`prepare_cff_programs`で閉包済みの実CFF glyphを評価する。全fontが、実admitted tableの一face一instanceとface ID順に一致することを評価前に確認する。別displayは拒否する。CFF /2のsource SHA/GID cacheを使うので、同じ元fontのface aliasや再要求でType2処理をやり直さない。TrueType書き出しと同じrecord/spool/workから継続し、途中失敗でも成功済みcacheとType2 operation/outline segmentの消費を戻さない。戻り値の統計は観測値であり、部分埋め込みbytesやPDF receiptではない。

共通Type2 kernelに確保前callbackを追加し、operand stack・call frames・輪郭Vecの拡張を事前に記録する。輪郭容量は明示的に4から倍増し、拡張分を予約してから確保する。path operatorは固定長48要素のstack上コピーを使い、元operand Vecの容量を保持する。従来の`mem::take`で容量を捨てて次のoperand列で無課金の再確保を起こす経路をなくした。既存/1と/2も同じ実行kernelを使い、元のType2解釈・FD選択・advance・輪郭・canonical charstringを維持する。

`evaluate_with_charge`と`prepare_closure_with_charge`は、文書の累積予算とType2専用operation/segment上限を両方維持する。operandを含む各operationは最大48 operandの処理を含めてworkを課金し、cache lookup/insertと保持nodeにも課金する。失敗したglyphをcacheへ入れず、再試行でも確保済み容量や評価試行の消費を返さない。CFFの実subset binary出力はまだこの文書予算へ接続しておらず、既存の無課金writerをbook-2で直接呼ぶ代替経路は追加していない。

元原ノ味を含むフォントcrate全62 tests成功（4.04秒、`/private/tmp/typaxis-book-v2-cff-evaluation-original-final.log`）。元23,060 glyphの実評価、FD/ローカル・グローバルsubroutine、元subset/UVS/vertical/admission、source alias cache共有を含む。追加したkernel試験は、複数path operator後もoperand容量を保持し、輪郭4→8の事前確保、record/byte/work上限ちょうど・1不足、輪郭/canonical charstring一致と失敗時の消費保持を確認した。ignoreの8件を含めて明示的に元原ノ味を渡して実行しており、ignoreを成功件数に数えた結果ではない。

この実装を全体完了とは扱わない。book-2 CFF binary writer、CID/ToUnicode/ActualText/font object、navigation/構造/Artifact/PDF、空段落flow/destination、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportと全巻/公開/管理host/独立/人手受入は引き続き残る。


最終統合69 tests成功（12.70秒、`/private/tmp/typaxis-book-v2-font-program-final.log`）。実font使用のsource SHA/GID集合とCFF cache件数、再要求時のoperation/segment不変・lookup work加算、record/spool/work上限ちょうど・1不足、先行work、同一byteでも別displayの拒否を確認した。Type2最初の試行に到達するwork境界を探索し、途中失敗で部分glyphがcacheへ入らず、再試行でも消費と確保記録が残ることを確認した。同じbuilderで実TrueType bytesを保持したままCFFを評価し、TrueType再出力とCFF cache再利用を行う混在ケースも成功した。

元原ノ味を明示指定した共有resource subset回帰も5 tests成功（2.80秒、`/private/tmp/typaxis-book-v2-font-kernels-original-final.log`）。新しい実TrueType writer、従来CFF subset、元CFF /3 shaping→subsetと独立font parseを含み、この実行ではignoreを残していない。

統合の実行コマンドは§14.136と同じ元原ノ味・Arial Unicodeの環境変数と`cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。font全試験は元原ノ味の環境変数を指定し、同manifest/locked/target-dirで`cargo test -p typaxis-font --lib -- --include-ignored`。共有subset回帰は同じ原ノ味指定で`cargo test -p typaxis-resources --lib subset -- --include-ignored`。


共通CFF kernel変更後の既定feature CLI text回帰29 testsも成功（5.41秒、`/private/tmp/typaxis-book-v2-font-program-legacy-text-final.log`）。同manifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis text`を実行し、従来の本文・脚注・番号・anchor・font/text/構造・PDF probeを確認した。最終4コマンドにcompiler warning/errorはなく、全プロセスが終了した。差分の空白検査も成功した。book-2 CFF binary/PDFへの接続と全体受入は引き続き未完である。


### 14.139 book-2の実CFF部分埋め込み

§14.138で残したCFF binary書き出しを、`BookV2FontSelectionBuilder::write_cff_font`へ接続した。実displayの閉包に属するCFF fontだけを受け取り、全faceの累積評価を先に完了し、そのsessionの評価済みglyphから`BookV2CffSubset`を生成する。元closed-fontを借用し、実OpenType/CFF bytes、dense mapping、元advance、metrics、name、閉包とbinaryに結びつくfingerprintを保持する。別display・存在しないfont index・TrueType指定を明示的に拒否する。失敗時にも元sessionを戻し、評価済みcacheや消費予算を破棄しない。

共有charstringエンコーダーは同じsink処理を計数器とVecに用いる。元輪郭から実byte長を確保なしで求め、累積charstring byte上限とspool/workを検査してから、必要な容量を確保する。以前の最大31 bytes/commandという容量を実出力長に置き換えたが、Type2の数値・path命令と出力bytesは同じである。記録・spool・workは実glyph、charstring、cmap/UVS、table構築、SFNT出力とidentity文字列の確保前に課金する。

CFF INDEX/charset/FDSelectの上限は実glyph数とcharstring長から計算する。小さいDICT/INDEXの収束処理は既存の最大32回に対する容量を計上する。元font全体のbytesや最大許容subsetサイズを倍して容量とする方法は使わない。format-12 cmapの作業列は元base mapの実件数に基づき、UVSのselector nodeと値配列は実際に保持する値と拡張分を事前予約する。coverage/tree走査・UVS整列もworkへ課金する。family/nameと保持tableは元データ長から計数し、最終SFNTは実table長でfont-byte上限を確認した後に確保する。

`Cff1SubsetSessionV2::write_prepared_subset_with_charge`は評価済みglyphしか書き出さない。不足cacheの検出時にもType2を暗黙に実行しない。従来の`subset`は同じ共有writerへ接続し、元の公開挙動を維持する。元原ノ味の既知subsetは5,052 bytes、SHA-256 `3b9ddbc2e0415a302c95f550113ddacfcc60d2f6fb219ad7f9296753f8823463`のままである。font-byte上限5,052では成功し、5,051では最終SFNT buffer確保前に拒否する。record/byte/work上限ちょうど・1不足と消費保持、既存の独立font parse・cmap/UVS・輪郭照合も成功した。

元原ノ味を明示的に渡したfont crate全62 tests成功（4.24秒、`/private/tmp/typaxis-book-v2-cff-writer-font-tests.log`）。このうちignore対象の8件も実行し、元23,060 glyphの評価と元source/alias/FD/UVS/vertical/subsetの検証を含む。実CFF書き出しを加えた統合69 tests成功（11.19秒、`/private/tmp/typaxis-book-v2-cff-subsets-tests.log`）。実source/instance/GID/hmtx/UPM/name、再生成bytes/metrics/fingerprint、Type2 cache消費不変、上限ちょうど・1不足、先行work、途中失敗と再試行を検証した。全cmap/UVSの走査を含むwriter試験は実測するwork容量を十分に設定し、実装側の上限を無効化していない。

### 14.140 book-2全fontの実プログラム集合

`write_font_programs`は実閉包に含まれる全fontを一つの`BookV2FontPrograms`へ書き出す。全CFF faceの累積評価後、全font slotの容量を先に確保し、実font順序でTrueType/TTCとCFFをそれぞれの共有writerへ渡す。戻り値は元閉包を借用し、全fontの実bytes/name/hash/元advance/dense GID/metricsを保持する。CIDやPDF計画が別displayのfont列や未出力のglyph表を代用する入口にはしない。

`BookV2FontProgram`は実部分埋め込み結果から読み取る共通viewを提供する。advanceは元font単位を維持し、metricsは両形式の実結果から同じPDF単位へ投影する。CFF /2はadmissionでUPM 1000を要求するので、元advanceを追加の仮scaleで変換しない。集合fingerprintは元閉包と全fontプログラムに結びつき、会計counterに依存しない。途中の失敗は完全なfont集合を返さず、予約済み容量とCFF cache消費を維持する。

全font集合まで加えた最終統合69 tests成功（5.11秒、`/private/tmp/typaxis-book-v2-all-font-programs-tests.log`）。本文・native数式・番号・marker・脚注・table再掲・元Harano+TTC混在の実font列について、元closed-font pointer、format signature、hash/name、全glyphのdense番号・advanceとmetrics、再生成byte/fingerprint一致を照合した。集合全体のrecord/spool/work上限ちょうど・1不足、先行work、同じbytesの別display拒否、途中失敗と再試行時の消費保持を確認した。空font集合にも同じowner/hash/accounting規則を適用する。

実font binaryの生成は接続したが、CID/ToUnicode/ActualText/font object、navigation/構造/Artifact/PDF、空段落flow/destination、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportと全ての全巻/公開/管理host/独立/人手受入は引き続き必要である。全体goalを完了とは扱わない。


最終共有resource subset回帰5 tests成功（2.75秒、`/private/tmp/typaxis-book-v2-cff-writer-resource-tests.log`、元原ノ味専用試験も実行）。既定feature CLI text回帰29 tests成功（1.92秒、`/private/tmp/typaxis-book-v2-cff-writer-legacy-text-tests.log`）。既存の実本文・脚注・番号・anchor・font/text/構造・PDF probeが成功し、今回の共有CFF書き出し変更が従来経路を壊していないことを確認した。

最終統合コマンドは、§14.138と同じ元原ノ味・Arial Unicodeの環境変数を指定した`cargo test --manifest-path workspace/Cargo.toml -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。font全試験は元原ノ味指定で同manifest/locked/target-dirの`cargo test -p typaxis-font --lib -- --include-ignored`、共有subset回帰は`cargo test -p typaxis-resources --lib subset -- --include-ignored`。既定回帰は同manifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis text`。最終4コマンドにcompiler warning/errorはなく、全プロセスが終了済み。元fontと正式元全巻packageは変更していない。CID/PDFと全体受入は引き続き未完である。

### 14.141 book-2の実CIDと出現単位の文字抽出

`BookV2FontSelectionBuilder::plan_cids`を実`BookV2FontPrograms`へ接続した。実選択・元font・subset GID・使用glyphを同じownerから照合し、使用glyphの昇順にCID 1以降を割り当てる。.notdefとTrueType複合部品は使用CIDへ混ぜない。TrueTypeは実subset GIDを対応表に保持し、CFFではdense CIDとsubset GIDの一致を確認する。widthは元advanceと実UPMから1000単位へ丸める。全font/binding/出現/CID slotを確保前に予約し、先行programのrecord/spool/workを継承する。

ToUnicode候補は単一scalar・単一glyphの観測から決め、同じglyphに異なるscalarがあれば曖昧な状態を維持する。元のscalar列とCID列の抽出結果を出現ごとに比較し、異なる場合だけ元sourceのActualTextを借用する。合字・複数glyphへの置換・IVSのために元文字列を推測しない。旧TrueType計画と共有するのは純粋なUnicode候補の統合関数だけであり、旧receiptを新ownerへ変換しない。別displayの同一bytes/fingerprintも拒否する。

実GSUB fixture `samples/machine-package/staging/production-book-1/vmb-book/cid-font/extraction.ttf`を既存のテスト用TrueTypeから生成した。A/Bが同じglyphになる`ABA`では曖昧なToUnicodeを出さず元A/B/Aを保持する。`fi`は一つのglyph、`X`はD/Eの二つのglyphへ実shapingされ、それぞれ元fi/XをActualTextとして保持する。fixtureは固定timestampで再生成でき、SHA-256は`b296ee1acf2e43a6f7f8e33331532ab899f653fd1cfc6c4a5468c7f07f4cd7bb`。元原ノ味・システムfontの改変ではない。

CID統合70 tests成功（5.47秒、`/private/tmp/typaxis-book-v2-cids-shaping-tests.log`）。実元font pointer、全使用glyph/CID/subset対応、width、独立set oracleによるUnicode候補と元文字列の完全復元を確認した。元Harano/TTC混在、IVS、Arial双方向式番号、native数式、番号・marker・脚注・table再掲にも同じ検査を適用する。再生成fingerprint、record/spool/work上限ちょうど・1不足、先行work、別owner拒否、全slot予約後の失敗と再試行時の消費保持を確認した。CID準備はType2再評価を行わない。

### 14.142 book-2のPDFフォント補助データ

`typaxis-pdf`の非公開feature経路へ`BookV2FontStreamBuilder`を追加した。実CID計画を借用し、各fontのToUnicode、TrueTypeのCIDToGIDMap、CFFのCIDSet、および必要な出現のBOM付きUTF-16BE ActualText tokenを生成する。CFFのCIDSetは.notdefを含む実dense glyph数と照合する。ActualTextをどのmarked-content範囲へ結びつけるかは、後続の構造・描画ownerが決める。親Formulaの置換範囲と重複するActualTextをこの段階で発行しない。

物理byte encoderは従来PDFと共有する。ToUnicodeは最大100件ずつ出力し、参照配列を追加確保せずclone可能なiteratorを最大二度走査する。UTF-16のsurrogate pair、CIDToGIDの先頭0、CIDSetの末尾bitを従来と同じ規則で出力する。新経路は同じencoderによる確保なしの計数後、実payloadと全metadataの容量を予約し、同じencoderで実byteを生成する。record/spool/output/workを累積し、途中失敗でも予約と消費を戻さない。fingerprintは元CID計画・実byte・range情報に結びつき、予算counterに依存しない。

最初の補助データ統合70 tests成功（7.25秒、`/private/tmp/typaxis-book-v2-font-streams-tests.log`）。独立したCMap/UTF-16 decoderで実byteを読み、bfchar件数、重複CID、元文字列復元、実subset GID、CFFの全CIDSet bitと余分な末尾bitを検査した。record/spool/output/work上限ちょうど・1不足、先行work、再生成、計数終了前と全容量予約後の失敗境界、再試行時の消費保持も確認した。空font集合も同じ規則に従う。最終owner/limits検査を加えた統合結果は追記する。

共有PDF crate全87 tests成功（1.16秒、`/private/tmp/typaxis-book-v2-font-streams-pdf-final.log`）。元原ノ味の実PDF試験も明示実行した。101件のCMap・空mappingの除外・補助面IVS・BOM有無・CIDSetのbyte境界、出力容量ちょうどと1不足の回帰を追加した。最初の試験コードでCid constructorのOptionをunwrapしていないコンパイル誤りを修正後の成功結果である。既定feature CLI text回帰29 tests成功（2.16秒、`/private/tmp/typaxis-book-v2-font-streams-legacy-tests.log`）。従来の実本文・脚注・番号・anchor・font/text/構造/PDF probeを含む。

実行には`--manifest-path workspace/Cargo.toml --locked --target-dir /private/tmp/typaxis-vmb-book-build`を使う。統合は§14.138の元原ノ味とArial Unicodeの環境変数を指定した`cargo test -p typaxis-cli --bin typaxis --features book-v2-staging book_v2_resources -- --include-ignored`。PDF回帰は元原ノ味を指定した`cargo test -p typaxis-pdf --lib -- --include-ignored`。既定回帰は`cargo test -p typaxis-cli --bin typaxis text`。

CIDとフォント補助byteは生成するが、book-2 font object・本文content stream・navigation/構造/Artifact・最終PDFの全体接続は未完である。空段落flow/destination、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportと全巻/公開/管理host/独立/人手受入も残る。全体goalや全巻PDFの成功とは扱わない。

最終統合70 testsも成功（5.58秒、`/private/tmp/typaxis-book-v2-font-streams-final.log`）。元CID/source ownerに結びついたeffective limitsとは異なる上限を渡した場合のIdentity拒否を追加し、既存の全読み戻し・容量境界を再確認した。最終統合・共有PDF・既定CLI回帰にcompiler warning/errorはなく、全プロセスは終了済み。実装・検証台帳へ同じ結果と未完範囲を記録した。

### 14.143 book-2の実フォントPDFオブジェクト

`BookV2FontObjectBuilder`を実`BookV2FontStreams`へ接続した。実fontごとにType0、CIDFont、FontDescriptor、実font program stream、ToUnicode stream、CID補助streamの6オブジェクトを生成する。TrueTypeはCIDFontType2・FontFile2/Length1・CIDToGIDMap、CFFはCIDFontType0・FontFile3/OpenType・CIDSetを用いる。name・metrics・width・元program bytesは実source ownerから取得する。CFFのWは.notdefを含むdense幅、TrueTypeのWは実使用CIDの幅を保持する。

既存PDFのfont dictionaryを`font_dictionary`の共有field visitorへ移した。既存経路は同じfieldから従来のPdfDictionaryを構成し、book-2は同じfieldを直接byteへ書く。辞書項目の順序、font形式別の項目、CID幅の表現、16.16 italic angleの形式別規則を共有する。name escapingと固定stackの数値表記も従来writerと共有する。book-2用に旧font receiptや旧object graphの権限を発行する変換は行わない。

新builderは指定された連続object番号の非0・桁溢れ・`max_pdf_objects`を先に検査する。実CID/resource ownerに結びつくeffective limitsと累積record/spool/output/workを継承し、確保なしの同じencoderで全objectの実長を計数する。全metadataと実出力bufferを予約してから生成し、各objectのid・役割・font index・byte範囲を保持する。戻り値はfontだけのobject fragmentであり、後続の全体PDF ownerが番号の重複なし・page resource参照・構造・全体graphを閉じる必要がある。これだけで公開book-2のPDF出力権限にはならない。

ToUnicodeとCID補助bytesはsource段階でfinal-output容量へ既に計上されているため、初回のobject予約でその分だけ控除する。コピー先のspool容量は全量を改めて課金する。ActualTextは後続本文のための予約として残す。object予約後はこの控除を消費済みにし、途中失敗と同じbuilderの再試行でも再利用しない。失敗した生成は完全なobject集合を返さず、予約と消費を戻さない。fingerprintは元stream owner、開始object番号、実bytesと全object範囲を結ぶ。

最初の統合70 tests成功（5.78秒、`/private/tmp/typaxis-book-v2-font-objects-tests.log`）。全6役割の連続id/範囲、実source pointer/name/font種別、参照先、実program/ToUnicode/補助streamの長さと内容、metrics、再生成bytes/fingerprintを照合した。record/spool/output/workの上限ちょうど・1不足、先行work、開始番号の変更によるfingerprint変化、最大object番号の境界、予約後失敗と再試行時の消費保持を確認した。最終版では確保前後のwork境界探索と異なるeffective limitsの拒否も追加した。

試験時だけ`TYPAXIS_BOOK_FONT_OBJECTS_PROBE`を指定すると、実font objectを空の非タグ付きpageに入れた独立parse用PDFと、実sourceから取得した期待値JSONを保存する。このpageは本文配置・構造・PDF/UAの実装を代用しない。最初の21 probes（TrueType 18 font、CFF 4 font）を`tools/verify_book_v2_font_objects.py`でpypdfのstrict readerへ渡し、object参照、W配列、metrics、元埋込みprogram SHA、ToUnicode、CIDToGIDMap/CIDSetを照合した。TrueType/CFFそれぞれのprogram 1 byte改変とCIDFont種別改変の計4件を拒否した。

共有PDF全87 tests成功（1.11秒、`/private/tmp/typaxis-book-v2-font-objects-pdf-tests.log`）。元原ノ味の実PDF試験も明示実行している。実行は元原ノ味の環境変数と`cargo test --manifest-path workspace/Cargo.toml -p typaxis-pdf --lib --features book-v2-staging --locked --target-dir /private/tmp/typaxis-vmb-book-build -- --include-ignored`。統合は§14.142と同じ元原ノ味・Arial Unicode指定で、任意のprobe保存先を加えた同節のCLI統合コマンドを使用する。独立検証はpypdfを持つPythonで`tools/verify_book_v2_font_objects.py <probe-directory> --self-test`を実行する。最終統合・既定CLI回帰・独立再検証は完了後に追記する。

フォントオブジェクトは実生成するが、book-2本文content stream、navigation/構造/Artifact、全体object graphと最終PDFの接続は未完である。空段落、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻export、全巻/公開/管理host/独立/人手受入も残る。全巻goalの成功とは扱わない。

最終統合70 tests成功（7.20秒、`/private/tmp/typaxis-book-v2-font-objects-final.log`）。異なる上限のIdentity拒否と全容量予約の直前/直後のwork境界を含めて再確認した。最終probesを新しい`/private/tmp/typaxis-book-v2-font-objects-probes-final`へ保存し、独立検証は21 PDF・TrueType 18 font・CFF 4 font、改変拒否4件が成功（`/private/tmp/typaxis-book-v2-font-objects-independent.log`）。既定feature CLI text回帰29 testsも成功（2.70秒、`/private/tmp/typaxis-book-v2-font-objects-legacy-tests.log`）。同manifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis text`を実行した。最終統合・共有PDF・既定CLIにcompiler warning/errorはなく、全プロセスは終了済み。元fontと正式元全巻packageは変更していない。差分の空白検査も成功した。

### 14.144 book-2の実glyph・罫線描画命令

`BookV2TextCommandBuilder`を実`BookV2FontObjects`へ接続した。元body displayの順序で、本文cluster、リスト/脚注marker cluster、式番号cluster、native数式glyph/ruleと脚注separatorを処理する。CID計画の元出現を同じpaint/slot順のcursorで照合し、全使用出現を一度ずつ消費する。実glyphのx/y、font instance/size、CIDをそのままTm/Tjへ渡し、native rule・separatorは元Rectをre/fへ渡す。元のshapingや位置を再計算しない。

各commandは元paint index、slot、CID使用index、page indexと実byte範囲を保持する。画像・vector数式のpaintには、このglyph/rule contributionの空範囲を明示的に保持する。存在しないpaint/commandはNone、存在するが担当する命令がないpaintは空sliceを返す。別途画像/vector contributionを接続するまで、これを全本文の描画結果とは扱わない。

raw commandはActualText/MCID/Artifactやq/Qを含めない。必要な出現のActualText tokenを元font-stream ownerから別に借用できる。後続のmarked-content ownerが、段落・数式・繰り返しheaderの正しい範囲へ結び付けるためであり、元の文字を捨てる処理ではない。ページ全体のY反転も後続page ownerが保持する。glyphは従来と同じ`1 0 0 -1 x y Tm`を用いる。

`text_encoding`へ物理的なfont選択・glyph・ruleの命令表記を共有化した。従来本文/脚注/native数式も同じkernelを使用し、既存のwrapperと元receipt検証は維持する。座標はi64の16.16値をi128の正確な10進数へ変換し、共有の固定stack formatterで出力する。glyphごとにformat!で一時Stringを作らない。i64最小/最大、負の1単位、整数境界、1/65536単位の座標とCIDの大文字hex表記を従来canonical関数・既知byte列と照合した。

builderは元font-objectのrecord/spool/output/workを継承し、実paint/command数、全range metadataと実出力長を計数する。確保なしの同じencoderを先行実行し、全容量を予約後に実byteを生成する。sourceのActualTextはここでcopy/出力しないので、glyph/rule command全量だけを新たにoutputへ課金する。fingerprintは元font objects、命令bytes、全出現/page/range対応を結ぶ。失敗・再試行で容量や消費workを返さない。

共有PDF全88 tests成功（1.35秒、`/private/tmp/typaxis-book-v2-text-commands-pdf-tests.log`）。元原ノ味のPDF試験を明示実行した。コマンドは§14.143と同じ元原ノ味指定、manifest/locked/target-dirを使う`cargo test -p typaxis-pdf --lib --features book-v2-staging -- --include-ignored`。統合では、独立の10進→16.16 decoderで実命令の全座標/CID/font/resetを元displayへ照合し、全出現消費、範囲、ActualText借用、累積予算境界・再試行・異なる上限の拒否を検査する。統合と既定回帰の完了結果は追記する。

任意の試験環境変数`TYPAXIS_BOOK_TEXT_COMMANDS_PROBE`は、実font objectsと実commandを独立parse用の試験PDFへ保存する。試験envelopeは単一の検査pageで、画像、元ページ分割、親Formulaの置換、タグ・navigationを実装するものではない。`tools/verify_book_v2_text_commands.py`はpypdfで実operatorを読み、元期待値のfont resource参照、size、Tm、CID、re/fと出現ActualTextを照合する。実GSUBのABA/fi/X fixtureはpdftotextによる元全文の抽出も検査する。全巻やPDF/UAの証拠にはしない。

book-2画像/vector命令、全本文marked-content・navigation/構造/Artifact・最終page/object graphとPDFは未完である。空段落、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻export、全巻/公開/管理host/独立/人手受入も残る。

統合70 tests成功（5.39秒、`/private/tmp/typaxis-book-v2-text-commands-tests.log`）。元原ノ味/TrueType/TTC、Arial双方向式番号、native数式、脚注・marker・繰り返しtableの実描画を含む。全glyph・ruleと元16.16座標、CID、font、page/slot/使用出現を照合し、全範囲と累積record/spool/output/workの上限ちょうど・1不足、先行work、予約直前/直後・途中失敗/再試行、異なるeffective limitsの拒否が成功した。

独立検証は21 PDF、1,671 glyph、59 rules、7 ActualText scopesを照合し、実GSUB fixtureのpdftotext全文抽出1件と、glyph行列/水平scaleの改変拒否2件も成功した（`/private/tmp/typaxis-book-v2-text-commands-independent.log`）。実行はpypdfを持つPythonで`tools/verify_book_v2_text_commands.py /private/tmp/typaxis-book-v2-text-commands-probes-01 --pdftotext /opt/homebrew/bin/pdftotext --self-test`。probeのsingle-page検査envelopeは正式な本文page/構造の実装ではない。

既定feature CLI text回帰29 tests成功（2.38秒、`/private/tmp/typaxis-book-v2-text-commands-legacy-tests.log`）。同manifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis text`を実行した。統合は元原ノ味・Arial Unicodeと上記probe環境変数を指定し、§14.143と同じCLI feature統合コマンドを使用した。統合/共有PDF/既定CLIにcompiler warning/errorはなく、全プロセスが終了済み。元fontと正式元全巻packageは変更していない。画像/vector命令と全体PDF/全巻受入は引き続き未完である。

### 14.145 book-2の実画像使用と共有リソース選択

`BookV2BodyDisplay::image_use`を追加し、通常Figureとvector数式の各paintから、実admitted image・元owner・page・viewport/matrix/colorを借用する。native数式、文字、marker、番号、separatorは画像使用を返さない。通常画像とvector数式の両方で元resource ledgerの実pointerを確認する。元配置とlogical IDは共有リソースから分離して保持する。

`BookV2FontSelectionBuilder::select_images`は実displayから選択された画像だけを集め、media・source SHA・vector parser/IR識別子とIR fingerprintをkeyとして共有する。同じkeyの元bytesと寸法を照合し、実使用logical IDの最小値を代表に選ぶ。通常Figureと数式、alias、繰り返しheaderでも個々の使用は元paint順に保持する。全slotを確保前に課金し、font選択と共有するfallible heapsortの比較・交換も累積workへ課金する。別displayの同じfingerprintを権限として受け入れない。

画像選択と次節のraster生成を本文font/textの先行record/spool/workへ接続した統合71 tests成功（6.17秒、`/private/tmp/typaxis-book-v2-image-resources-final.log`）。実PNG/JPEG/SVGのlogical ID 1→0という二配置が一つの代表resourceを参照し、二つのownerと出現順を保持する。元原ノ味・Arial Unicodeを明示した§14.143のCLI feature統合コマンドで実行した。

### 14.146 book-2の実PNG/JPEG出力データ

`write_raster_programs`を実画像選択へ接続した。PNGは共有のbounded decoderで色と必要なalphaを分離し、固定したFlate encoderでそれぞれ圧縮する。decoder・色/alpha buffer・各deflater workspaceを確保前に課金し、圧縮出力も各chunkの予約前にrecord/spool/workへ加える。level 6の探索係数128を入力サイズへ適用するwork重みは、正確なCPU命令数を意味しない。JPEGは実admission attestationと元hash・寸法・normalized stream hashを照合し、検証済みnormalized bytesを借用する。同一選択resourceを配置ごとに再encodeしない。SVGのslotは明示的にNoneで、raster化しない。

実payload bytes/hash、元attestation pointer、alphaの寸法/8bit、再生成、record/spool/work上限ちょうど・1不足、先行work、失敗/再試行の消費保持、別display拒否を上記71 testsで確認した。既定featureの画像回帰5 testsも成功（2.00秒、`/private/tmp/typaxis-book-v2-raster-legacy-tests.log`）。実行は同manifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis raster`。

任意の`TYPAXIS_BOOK_RASTER_PROGRAMS_PROBE`は元mediaと実圧縮payloadを試験時だけ保存する。Pillowとzlibを使う`tools/verify_book_v2_raster_programs.py`で、最終5 probes（PNG 3・JPEG 2・alpha 3）の元画素と実payloadを独立照合し成功した（`/private/tmp/typaxis-book-v2-raster-programs-independent.log`）。実行はbundled Pythonで同scriptへ`/private/tmp/typaxis-book-v2-raster-programs-probes-final`を渡す。これらは画像データの証拠であり、book-2 Image/SMask object・page command・最終PDFの実装を代用しない。

### 14.147 book-2の実SVG Formプログラム

`BookV2VectorProgramBuilder`は実raster/画像選択ownerを借用し、SVGだけについて実IRから再利用可能なForm contentとExtGState辞書を生成する。実選択imageとIRの両pointer、source SHA、IR fingerprint、実byteを保持して結び付ける。PNG/JPEGのslotはNoneで、画像種別を曖昧に扱わない。alpha pairは全draw分を確保前に予約し、fallible heapsortと重複除去でcanonicalな順序へ集約する。各drawは対応する`/GS<index>`を参照する。

従来`safe_vector_v2`にあったroot clip/viewBox、clip path、transform、line style、色、quadratic→cubic変換とpath命令を`vector_encoding`へ共有化した。callerがExtGStateの参照と予算を所有し、旧Form計画やreceiptは新ownerへ変換しない。固定小数の数値表記はstack bufferを用い、一時Stringを確保せず計数・生成の両方を同じ処理で行う。新経路は全metadataと実payload長の予約後にbyteを生成し、record/spool/output/workの先行消費を継承する。FormにMCID・Alt・ActualText・Langは置かず、各配置の意味範囲は後続stageに残す。

独立レンダリングは実装の誤りも検出した。塗りalpha 0.75・線alpha 0.5のSVGをPDFの`B`で同時描画すると、塗りと線の重なりがSVGと異なり、144 DPIで平均RGB誤差7.52、32超の差を持つ画素6.90%となった。book-2は同じpathを塗り`f/f*`、線`S`の順に処理してSVGの合成を保持する。既存の凍結された経路は従来operatorを保持する。比較閾値は当初の平均誤差2/255以下・32超の画素3%以下から変更していない。

比較用の`tools/verify_book_v2_vector_programs.py`は実contentをpypdfでparseし、canonical IRから独立に計算したroot/clip/transform・全path座標・paint/state参照と照合する。integer/Fractionでties-to-evenを計算し、全命令数と順序を検査する。試験envelopeにだけFormを載せてMuPDFと元SVGのlibrsvg出力を144/288 DPIで比較する。envelopeはbook-2の実Image/Form object graph・元page assembly・構造を代用しない。rendererはMuPDF 1.28.2、rsvg-convert 2.62.3を明示照合する。

塗り/線修正後の統合71 tests成功（7.25秒、`/private/tmp/typaxis-book-v2-vector-programs-compositing.log`）。最初の独立5 probesはIR /1・/2、21 draws、6 alpha states、473命令を照合し、root clip/GS参照改変10件を拒否した。全10レンダリング比較が当初の閾値内で成功した（`/private/tmp/typaxis-book-v2-vector-programs-independent.log`）。最初の新fixtureは許可されないclipPath transform属性を使ってadmissionに拒否されたため、許可構文のfixtureへ修正した。元の実SVGやadmission規則を緩めていない。

元画像owner/fingerprintの結合とpayload予約直前/直後のwork失敗・再試行検査を追加した最終統合、共有PDF・既定CLI回帰、最終独立probesの結果は完了後に追記する。book-2 raster Image/SMask・vector Form/ExtGStateの実object、画像配置命令、全本文marked content/navigation/構造/Artifact・page/object graph・最終PDFは未完である。空段落、動的label/行/ページ収束、再帰/脚注table、残る著者kind、正式元全巻exportと全巻/公開/管理host/独立/人手受入も残る。

最終統合71 tests成功（8.19秒、`/private/tmp/typaxis-book-v2-vector-programs-owner-final.log`）。実選択image/IR pointer、source SHAを含む再生成fingerprint、全state参照、rasterとの排他、累積record/spool/output/workの上限ちょうど・1不足、先行work、異なるeffective limitsの拒否を確認した。最初のpayload予約に到達するwork境界を探索し、直前/直後の失敗と再試行で消費済みoutput/spool/recordが保持されることも確認した。空vector集合も同じbuilderを通る。

最終独立検証は5 probes・473命令・21 draws・6 states・改変拒否10件が成功し、144/288 DPIの全10比較も成功した（`/private/tmp/typaxis-book-v2-vector-programs-independent-final.log`）。最大の平均RGB誤差は1.7278/255、32超の差を持つ画素の割合は1.6667%。pypdf 6.10.0、Pillow 12.3.0、MuPDF 1.28.2、rsvg-convert 2.62.3を記録した。実行はbundled Pythonで`tools/verify_book_v2_vector_programs.py /private/tmp/typaxis-book-v2-vector-programs-probes-owner-final --self-test --mutool /opt/homebrew/bin/mutool --rsvg-convert /opt/homebrew/bin/rsvg-convert`。

共有PDF回帰89 tests成功（1.27秒、`/private/tmp/typaxis-book-v2-vector-programs-pdf-final.log`）。元原ノ味を指定した§14.143と同じfeature付きPDF lib testコマンドで実行した。既定feature CLI vector回帰16 tests成功（1.78秒、`/private/tmp/typaxis-book-v2-vector-programs-legacy-final.log`）。同manifest/locked/target-dirで`cargo test -p typaxis-cli --bin typaxis vector`を実行し、管理hostと外部PDF/UA環境を要求する`machine_precomposed_vector_external`の1件は既定通りignoredである。これは未実行の全巻/外部受入を成功とするものではない。

統合は元原ノ味・Arial Unicodeと`TYPAXIS_BOOK_VECTOR_PROGRAMS_PROBE=/private/tmp/typaxis-book-v2-vector-programs-probes-owner-final`を指定した§14.143のCLI feature統合コマンドを使用した。最終統合・共有PDF・既定CLIにcompiler warning/errorはなく、全プロセスは終了済み。差分の空白検査も成功した。上記の画像object・配置命令・全体PDF接続と全巻受入は引き続き未完であり、全体goalは完了にしていない。

### 14.148 book-2の実Image/SMask/Form/ExtGStateオブジェクト

`BookV2ImageObjectBuilder`を実vector/raster/画像選択ownerへ接続した。各選択resourceに、PNG/JPEGのImageと必要なSoftMask、またはSVGのFormとcanonical alpha順のExtGStateを生成する。元image indexごとのobject範囲と参照先を保持し、連続object番号の非0・桁溢れ・最大番号を検査する。元resource ownerとeffective limitsは一致を要求する。logical aliasや再掲headerの各配置は、同じ代表Image/Formを参照できる。

Imageは実8bit正規化済みpayload・寸法・color spaceを使用する。PNGとalpha maskはFlateDecode、JPEGはDCTDecodeと実color kindに対応するColorTransformを指定する。JPEGのmaskや不整合なalpha寸法/形式を受け入れない。SVG Formは実BBox・content bytesとExtGState参照を持つ。元のImage/Formの物理辞書fieldsを`image_encoding`へ共有化し、従来経路の出力field順・spacing・成功byteを保つ。新object fragmentへ旧receiptや旧graphの権限を付け替えない。

実出力長を確保なしで計数し、全object/range metadataとbyte bufferを予約してから同じencoderで生成する。元vector programのcontent/alpha辞書は既にfinal-output容量へ計上されているので、初回のobject予約で一度だけ控除する。コピー先spoolは全量を課金する。予約後に控除を使用済みにし、途中失敗や同じbuilderの再試行で再利用しない。実source・開始番号・byte・全object/range対応がfingerprintに入る。

統合71 tests成功（6.22秒、`/private/tmp/typaxis-book-v2-image-objects-final.log`）。全object参照/role/range、元payload、共有resource、予算上限ちょうど・1不足、最大object番号、先行work、再生成、異なるlimitsの拒否を確認した。payload予約直前/直後のwork境界と再試行の消費保持も確認した。最初の統合では試験側が先行outputを引かず比較して失敗したため、累積予算の比較式を修正している。

任意の`TYPAXIS_BOOK_IMAGE_OBJECTS_PROBE`で実objectを空の検査page付きPDFに保存する。`tools/verify_book_v2_image_objects.py`がpypdf strict readerで全object/SMask/GS参照と埋込みbytesを読み、PNG/JPEGの画素とalphaをPillow/zlibで照合し、SVGの全operatorを元IRへ照合する。実objectを一画像ずつ試験pageへ参照した独立描画も比較する。試験pageは実書籍のpage assemblyや構造を代用しない。

最終独立検証は28 PDF、PNG 3・JPEG 2・vector 14・mask 3・alpha states 15、描画比較38件、geometry/payload改変拒否38件が成功した（`/private/tmp/typaxis-book-v2-image-objects-independent-final.log`）。極小rasterの画素比較はMuPDFの`-A 0`で補間を無効にする。最初の比較ではrendererの自動補間が境界色を混合したため、その条件を明示した。vectorは既存のAA条件・144/288 DPI・閾値を維持する。pypdf 6.10.0、Pillow 12.3.0、MuPDF 1.28.2、rsvg-convert 2.62.3を使用した。

共有PDF89 tests成功（1.28秒、`/private/tmp/typaxis-book-v2-image-objects-pdf-tests.log`）、既定CLI raster 5 tests成功（1.15秒、`/private/tmp/typaxis-book-v2-image-objects-legacy-raster.log`）、既定CLI vector 16 tests成功（1.96秒、`/private/tmp/typaxis-book-v2-image-objects-legacy-vector.log`）。vectorの外部管理host試験1件は既定通りignoredである。CLI統合・PDF libは§14.143の元font/feature/locked/target-dir指定を使い、probe変数を上記用途に置換した。独立検証はbundled Pythonで`tools/verify_book_v2_image_objects.py /private/tmp/typaxis-book-v2-image-objects-probes-final --self-test --mutool /opt/homebrew/bin/mutool --rsvg-convert /opt/homebrew/bin/rsvg-convert`を実行した。

この結果は画像object fragmentの生成であり、実page resourceとの全体結合、全本文marked content・navigation/構造/Artifact、最終PDF/全巻受入は別途必要である。

### 14.149 book-2の実画像配置命令

`BookV2ImageCommandBuilder`は実Image/Formオブジェクトと選択画像を借用し、各元paint・owner・page・resource参照に対応する配置命令を生成する。PNG/JPEGは実viewportの高さ反転、SVGは実配置行列とcurrentColorを使い、`BI<resource index>`で同じ代表resourceを参照する。共有画像でも配置と意味の出現は統合しない。q/Qで描画状態を隔離し、元座標を再計算しない。画像とvector配置の物理encoderを既存経路と共有した。MCID・Alt・ActualTextとページ全体のY反転は後続stageが担当する。

統合71 tests成功（8.42秒、`/private/tmp/typaxis-book-v2-image-commands-final.log`）。全配置の命令・実16.16座標・色・元owner/page・参照先を照合し、累積record/spool/output/work上限ちょうど・1不足、先行work、予約後失敗と再試行、異なるlimitsの拒否を確認した。独立pypdf検証は28 PDF、raster 11配置・vector 136配置・共有resource使用128件、改変拒否47件が成功（`/private/tmp/typaxis-book-v2-image-commands-independent-final.log`）。`tools/verify_book_v2_image_commands.py /private/tmp/typaxis-book-v2-image-commands-probes-final --self-test`をbundled Pythonで実行した。probeは実命令を載せた検査pageであり、元書籍のpage/構造の実装ではない。

共有PDF89 tests成功（1.25秒、`/private/tmp/typaxis-book-v2-image-commands-pdf-tests.log`）。既定CLIのproduction回帰では244件成功し、5,000個の異なるSVG Formと5,000配置での同一Form共有の両試験を含む。最初の一括実行では公開CLIが拒否する試験用font環境変数を設定したため5件が失敗し、保存済みjobを要求するignored試験1件にも入力を指定しなかった。通常環境での`machine_production_book_1`再実行は5件成功（2.04秒、`/private/tmp/typaxis-book-v2-image-commands-public-tests.log`）、元jobを指定した`production_common_driver_saved_vmb_job -- --include-ignored`は1件成功（1.36秒、`/private/tmp/typaxis-book-v2-image-commands-saved-vmb.log`）。244件の一括結果は`/private/tmp/typaxis-book-v2-image-commands-production-tests.log`に記録した。これは管理hostでの計測条件を固定した性能受入ではない。

保存済みの正式VMB小入力`/private/tmp/vmb-legacy-source-integrity-public/typaxis-body-2416664007`から、`/private/tmp/typaxis-book-v2-image-commands-legacy-vmb.pdf`を生成した。1ページ・86,609 bytes、SHA-256 `a1f90f2c6f1de613a92cbe106cfb1561c614e721701959668bcffbc9b643c51d`。pypdf strict readerで旧公開PDFとの本文content bytes、実Form content bytes、resource名と全文抽出の一致を確認した。共通driverと旧公開envelopeのobject割り当て順とXMP PDF/UA宣言が異なるため、PDF全体のbyte一致を主張しない。新book-2の正式全巻PDFやPDF/UA受入を代用しない。

通常feature CLI checkはwarning/errorなしで成功（24.31秒、`/private/tmp/typaxis-book-v2-image-commands-check-final.log`）。各cargoコマンドは§14.143と同じmanifest/locked/target-dirを使う。全プロセスは終了済み。全本文marked content・navigation/構造・page/object graphと最終PDF、元全巻export、残る著者kind・収束・table・空段落と全巻受入は引き続き未完である。

### 14.150 book-2の元paintに対応するmarked-content境界

`BookV2MarkedScopeBuilder`は実画像配置命令から同じ元displayを借用し、全paintを欠落・重複なく連続した意味範囲へ分ける。本文text clusterは同じ元owner・選択fragment・page・表示役割が連続する場合だけまとめる。数式・図・生成ラベル・独立式番号はそれぞれ固有の範囲を保持し、式番号をFormulaの置換文字列へ入れない。各semantic範囲へページごとに0から連続するMCIDを割り当て、元owner、paint範囲、fragment、pageとともに保持する。空pageの範囲も明示する。

繰り返し表ヘッダーのtext・native/vector数式・画像・ラベル・式番号は、実配置のrepeated-header役割に従いPagination/Header Artifactとなる。脚注separatorはLayout Artifactとなる。これらはMCID・Alt・ActualTextを持たない。元owner情報は診断と配置照合のために保持する。共有Image/Formの内部へ意味情報を追加しない。

開始のBDC辞書を実byteとして生成し、各範囲の終了EMCも出力予算へ計上する。native Formulaは元speech、vector Formulaは元のresolved ActualText、通常inline vectorは明示されたActualTextを使う。Figure/Formulaの元Altは将来のStructElem用に借用し、共有resourceへ移さない。任意の欠落したFormula置換を自動生成せず、整合性エラーとする。文字glyphごとの抽出補正、描画命令との挿入順、vector数式の非描画抽出anchor、StructElem/ParentTree・navigationと完全なpage/object graphは後続の未完実装である。

全scopeとpage rangeのmetadata、開始byte bufferを先に計数・予約する。元ownerの累積record/spool/output/workとeffective limitsを継承し、同じencoderの計数・実生成の両方へworkを課金する。source fingerprint、元owner/page/fragment/role、全paint/byte範囲と実命令byteをfingerprintへ結ぶ。失敗・再試行で予約済み容量や消費workを返さない。

統合71 tests成功（6.83秒、`/private/tmp/typaxis-book-v2-marked-scopes-tests-02.log`）。元原ノ味、TrueType/TTC、双方向式番号、native/vector数式、通常図、脚注と繰り返しheaderの全元paintを独立component記録へ照合した。範囲の被覆・最大連結、page/MCID対応、元Alt/ActualText、Artifact除外、上限ちょうど・1不足、先行work、再生成、途中失敗・再試行、異なるlimitsの拒否が成功した。初回コンパイルではPDF crateから未依存のpagination型を参照した点と、resolved ActualTextのOption型の扱いを修正した。依存を追加せず、元math displayから借用するaccessorを使用した。

`tools/verify_book_v2_marked_scopes.py`はpypdfで実BDC/EMCをparseし、辞書の全key・tag・MCID・Artifact種別・UTF-16BE ActualTextを元期待値へ照合する。28 probes、1,844 paints、565 scopes（semantic 422、繰り返しHeader 102、separator 41）、ActualText 64件、MCID/tag改変拒否565件が成功した（`/private/tmp/typaxis-book-v2-marked-scopes-independent.log`）。実行はbundled Pythonで`tools/verify_book_v2_marked_scopes.py /private/tmp/typaxis-book-v2-marked-scopes-probes-01 --self-test`。これは境界命令の検査であり、最終PDFの構造・抽出・PDF/UA受入ではない。

共有PDF89 tests成功（1.39秒、`/private/tmp/typaxis-book-v2-marked-scopes-pdf-tests.log`）。元原ノ味を明示した§14.143のfeature付きPDF lib testコマンドを使用した。CLI統合も同節の元font/manifest/locked/target-dir指定を使い、`TYPAXIS_BOOK_MARKED_SCOPES_PROBE`で上記検査用byteを保存した。最終page/構造の接続、空段落・動的収束・再帰/定義table・残る著者kind、正式元全巻export、公開/管理host/独立/人手による全巻受入は引き続き未完である。

通常feature CLIの`cargo check -p typaxis-cli --bin typaxis`も同じmanifest/locked/target-dirで成功した（1分19秒、`/private/tmp/typaxis-book-v2-marked-scopes-default-check.log`）。最終統合・共有PDF・通常CLIにcompiler warning/errorはない。全プロセス終了と差分の空白検査成功を確認した。

### 14.151 book-2の実marked本文と抽出用グリフの接続

`BookV2MarkedContentBuilder`は、実文字命令と前節の実scope・画像命令を結合する。すべての元paint、文字命令、画像命令を一度ずつ消費し、選択済みページとscopeの順序を保つ。空pageも保持する。ページ別・scope別のbyte範囲を公開し、各scopeのq/Q、BDC/EMCと実glyph/rule/Image/Form命令を生成する。最終page ownerがMediaBoxとページ全体のY反転を付けるため、この本文は元のY-down座標のままである。

本文textは実選択fragmentの元cluster文字列、生成ラベルは元marker、式番号は元の番号全文を一つのActualTextで包む。native/vector Formulaは親scopeの置換を使用する。内側のCID出現ごとの補正を重ねず、空白・合字・双方向番号を元の出現範囲に保持する。ArtifactへActualText/MCIDを付けない。文字を抽出すべきvector出現には、元viewport幅・高さ・xと実baselineを使う非描画グリフを置く。SVG自体を文字やrasterへ置換しない。

抽出用Type3 Font、非描画CharProc、ToUnicodeの実3オブジェクトも必要な場合だけ生成する。`BMA`から参照し、元font/imageの使用番号の後へ割り当てる。元フォントと画像の実object番号が衝突すれば拒否し、追加3個の上限ちょうど・1不足も検査する。グリフは描画命令を持たず、Tr=3も指定する。既存のPBA用辞書・CMap・命令を`semantic_anchor_encoding`へ共有化し、旧経路のbyte表記は維持した。

同じ元displayのpointerとlimitsを要求する。画像選択前のrecord/spool/work、およびvector出力前のoutputを保持し、先行する実文字命令の全消費を含むことを確認する。二つの独立消費の大きい方だけを採用する結合は許可しない。既に予約済みのraw文字・画像・scope命令と、今回置換するCID単位のActualText tokenは初回の最終buffer予約で一度だけ控除する。コピー先spoolと全page/scope/anchor/object metadataは全量を予約する。予約後の失敗・再試行で控除を再利用しない。

最初の統合71 tests成功（7.61秒、`/private/tmp/typaxis-book-v2-marked-content-tests.log`）。別display、先行予算の各1不足、異なるlimits、object衝突、追加抽出objectの上限境界を加えた最終統合も71 tests成功（6.14秒、`/private/tmp/typaxis-book-v2-marked-content-owner-final.log`）。実ページ別の全paint/命令の被覆、元文字列、非描画グリフの実geometry、予算の上限ちょうど・1不足、先行work、再生成と途中失敗・再試行を確認した。共有encoderの初回コンパイルではclosureの借用型推論を修正した。

`tools/verify_book_v2_marked_content.py`は実font/image/抽出objectと実本文を持つ検査PDFをpypdfでparseする。最終28 probes・107 pages・565 scopes、glyph 1,797・rule 59・画像配置147・抽出グリフ47、ActualText 358・Artifact 143を元期待値へ照合した。実GSUB fixtureの`ABA fi X D E f i`全文抽出1件、ページ行列・MCID・Tr・ActualText改変拒否91件も成功した（`/private/tmp/typaxis-book-v2-marked-content-independent-owner-final.log`）。

MuPDF 1.28.2とPillowで抽出グリフを除いた対照も描画し、対象40 pagesすべてでRGB画素が完全一致した。同じpypdf命令再serializationを両対照に適用し、数値表記の差を混入させていない。最初の試験補助に出たpypdfの未所属page編集のdeprecation warningは、writerへpageを所属させてから編集する処理へ修正した。最終独立検証にそのwarningはない。検査pageのMediaBoxは固定した試験envelopeで、元page master・StructTreeRoot・navigation・PDF/UA・全巻受入の証拠ではない。

共有PDF89 tests成功（1.13秒、`/private/tmp/typaxis-book-v2-marked-content-pdf-tests.log`）、通常feature CLI text回帰29 tests成功（2.36秒、`/private/tmp/typaxis-book-v2-marked-content-legacy-text.log`）。保存済みの正式VMB小入力からの共通driver試験1件も成功（1.48秒、`/private/tmp/typaxis-book-v2-marked-content-saved-vmb.log`）。出力`/private/tmp/typaxis-book-v2-marked-content-legacy-vmb.pdf`は前段階の共通driver PDFとbyte一致し、SHA-256は両方とも`a1f90f2c6f1de613a92cbe106cfb1561c614e721701959668bcffbc9b643c51d`である。

cargoは§14.143と同じmanifest/locked/target-dir、CLI feature統合とPDF libには元原ノ味を明示し、CLI統合にはArial Unicodeも明示した。最終probeは`TYPAXIS_BOOK_MARKED_CONTENT_PROBE=/private/tmp/typaxis-book-v2-marked-content-probes-owner-final`。独立検証はbundled Pythonで`tools/verify_book_v2_marked_content.py /private/tmp/typaxis-book-v2-marked-content-probes-owner-final --self-test --pdftotext /opt/homebrew/bin/pdftotext --mutool /opt/homebrew/bin/mutool`。保存済みVMB試験の入力は§14.149と同じで、PDF出力先を上記へ置換した。完全な構造tree・navigation・source page masterとglobal object graph/最終PDF、残るlayout/source機能と正式元全巻export・全巻受入は引き続き未完である。

通常feature CLI checkも成功（21.01秒、`/private/tmp/typaxis-book-v2-marked-content-default-check.log`）。最終CLI統合・共有PDF・既定CLI回帰/checkにcompiler warning/errorはなく、全プロセス終了と差分の空白検査成功を確認した。

### 14.152 book-2の元文書階層と実MCIDの対応

`visit_book_v2_structure`と`BookV2SourceStructureBuilder`を追加した。元のstyled bodyとnavigationを保持する実source flowから、元文書順の構造を取り出す。新形式を旧`StructureRegistryReceiptV2`や旧accessibility authorizationへ変換しない。文書・段落・見出し・inline階層、図・数式・番号・表・リスト・脚注を保持し、元node IDと生成slotの組で構造の所有者を区別する。

12種類のsemantic containerは元kindをそのまま保持し、quoteはBlockQuote、他11種はSectを標準PDF roleとして持つ。リストはLIの下にLblとLBody、表はTHead/TBodyの下にTRとTH/TD、captionはFigureの下にCaption、脚注の参照・定義はそれぞれReference/Noteの下にLinkとLblを持つ。式番号は元Formulaの独立したSpan子として保持し、FormulaのActualTextへ混ぜない。元language・source span・Figure/FormulaのAltは元ownerから借用する。Tableのrow/column/colspan/rowspan/head区分は元flowの実検証済みgridへ照合し、全tableを同じ順のcursorで一度ずつ消費する。

構造のkey lookupは予約済み配列をfallible heapsortで整列し、重複key、未解決parent、親より先の子、元language ownerや式番号の欠落を拒否する。親子・兄弟順とノードごとの本文出現の連鎖を保持する。実`BookV2MarkedContent`のすべてのsemantic scopeを元nodeまたは専用Lblへ一度ずつ対応させ、page/MCIDとgroup indexを保持する。繰り返し表headerと脚注separator Artifactは対応を持たない。同一ノードの複数page/fragmentでの出現は同じ構造ノードへ結ぶ。

source walkerはcollectionを確保せず、callerのwork制限に従って停止する。全metadataを計数してからrecord/spoolを予約し、実構造と対応配列を生成する。lookup比較・交換、元構造の走査、fingerprintの文字列chunk処理もworkへ計上する。元の本文で一度消費したraw payloadの出力控除は再導入せず、確定済み本文の累積予算を継承する。この段階は新たなPDF bytesを生成せず、output予算を増減しない。失敗・再試行で予約済みrecord/spoolやworkを返さない。

`PreparedBookV2Navigation::ast_node_count`は検証済みの元AST総数を保持する。文書だけでなくnative math、advanced page、metadata、outlineの既存消費を含み、構造の生成ノードはその総数へ加える。初回のAST境界試験でこの既存消費の一部を数えていなかった点を修正した。生成ノードを含む構造の深さも制限する。元languageを計算し直したり、本文に存在しない図・数式・文字を追加したりしない。

初回のCLI統合71件成功（5.72秒、`/private/tmp/typaxis-book-v2-source-structure-tests.log`）、構造depthとprobeを加えた統合71件も成功（5.53秒、`/private/tmp/typaxis-book-v2-source-structure-final-02.log`）。probe補助の初回compileではSerializeを持たないpackage wrapperを直接使った点を修正し、保持された元documentを保存するようにした。12種類のkindを試す構文fixtureには本文styleを明示した。最終構文ライブラリ全112 tests成功（1.27秒、`/private/tmp/typaxis-book-v2-source-structure-syntax-all-final.log`）。12種類のrole/元kind・languageの借用、source walkerの停止、生成構造込みのAST数と深さの上限ちょうど・1不足を含む。

`tools/verify_book_v2_source_structure.py`はprobeに保存した元documentを独立に走査し、意味階層、元kind/Alt/span/継承言語、生成slot、兄弟順と表gridを復元する。実group bytesはpypdfのContentStreamでparseし、実BDCのMCIDと構造対応、Artifactの除外、各ノードの本文出現列を照合する。これは最終StructElem/ParentTreeを持つPDFの検査ではない。

このregistryは元文書階層と実本文対応である。脚注定義は元のdocument子として保持しており、最終読み順への移動、Reference/Note関係、table Headers/IDTree、annotation/OBJR、StructElem/ParentTreeの実bytesは後続で接続する。source page master、最終global object graph/PDF、残るlayout/source機能、正式元全巻exportと公開・管理host・独立・人手による全巻受入は未完である。最終CLI統合・独立検証・通常feature checkの結果は追記する。

最終CLI統合71 tests成功（5.34秒、`/private/tmp/typaxis-book-v2-source-structure-owner-final.log`）。すべての親子・兄弟・本文出現連鎖、元language ownerと式番号の全件保持、元表grid、生成labelとArtifactの区別、異なるlimitsの拒否、record/spool/output/workの上限ちょうど・1不足、先行work、再生成と途中失敗・再試行を確認した。元原ノ味・Arial Unicodeを明示し、§14.143と同じmanifest/locked/target-dirとCLI feature統合コマンドを使用した。probeは`TYPAXIS_BOOK_SOURCE_STRUCTURE_PROBE=/private/tmp/typaxis-book-v2-source-structure-probes-final`。

最終独立検証は28 probes・975構造nodes・422の実MCID対応・143の除外Artifactを照合し、改変拒否170件が成功した（`/private/tmp/typaxis-book-v2-source-structure-independent-final.log`）。親・role・language・Alt・cell column・対応MCIDに加えて、実BDC bytes自体のMCID改変も拒否した。実行はbundled Pythonで`tools/verify_book_v2_source_structure.py /private/tmp/typaxis-book-v2-source-structure-probes-final --self-test`。通常feature CLI checkも同じmanifest/locked/target-dirの`cargo check -p typaxis-cli --bin typaxis`で成功した（0.07秒、`/private/tmp/typaxis-book-v2-source-structure-default-check.log`）。最終構文・統合・checkにcompiler warning/errorはなく、今回の全プロセス終了、対象ファイルのformatと差分の空白検査成功を確認した。元全巻package・元fontは変更していない。

### 14.153 book-2の脚注読み順・参照関係と同一ページの依存解決

`BookV2StructureRelationBuilder`を実marked本文のsource registryへ接続した。元のnode index・kind・language・source span・Alt・MCID対応を保持したまま、脚注のReference/Note/Link/Lblの相互関係と、実際に描画された参照・未配置定義を区別する。静的な戻り先はsource順で最初の描画済み参照とする。実ページ選択のfirst-reference provenanceを使って、本文から到達する脚注の木を作り、その木に循環がないことを後続の移動候補の選択前に確認する。定義のsource順に、自己の子孫へ移動しない最後の描画済み参照の段落内へNoteを移す。複数定義が同じinline branchを共有するときは参照owner・定義owner順で安定させる。すべての参照辺は残すが、循環する依存辺を構造木の親子関係にはしない。この決定的な一巡の移動は、全候補の固定点や大域最適な読み順を主張するものではない。

移動後の木は非再帰の走査で全nodeの一意な到達・親子/兄弟関係・実depth上限を検証する。元の定義順が参照順と逆でも、本文と脚注でlanguageが異なっても、未参照の定義内に別の参照があっても元の意味情報を保持する。リストのDecimal/Discと、実table grid上で列区間が重なるTHからTDへのHeaders関係も保持する。headのない表に見出しは捏造しない。ID文字列・IDTreeやStructElem bytesはこのownerでは生成しない。

§14.152の実Text描画に対する対応も修正した。inline脚注番号はMarkerではなくTextとして描画されるため、これをReference直下のgenerated FootnoteLabelへ結び付ける。通常のanchor参照は従来どおりReferenceに結び付ける。raw描画bytesは変更していない。実MCIDに対応するLblを使い、Reference/Note双方の描画済み判定を行う。新しい検証器は元document、実BDC、実MCIDと選択済みfirst-demand ownerから関係を独立に再構成する。first-demand自体を独立に再ページ配置した証拠ではない。

相互参照を通す実統合試験では、共通の脚注予約が入口時点の定義だけを配置し、定義内から新しく要求された脚注を開始できない問題も見つかった。book-2のjoint-fitへ、通常の予約がこの状態を残した場合だけ起動する、有界な依存探索を追加した。実測の合法な区切り候補をcost/end順に明示stackで探索し、選択したfragmentだけから需要とfirst-referenceを進める。後続定義が入らなければ候補を戻し、採用しなかった参照の需要も破棄する。同一ページでは各定義を高々一fragmentにし、残りを継続として保持する。循環参照は既に開始した定義を再帰的に配置し直さない。強制改ページ・区切れない参照・separatorと定義間spacing・領域上限を守り、候補数/record/work上限は失敗でも返却しない。探索打切りはエラーであり部分成功にはしない。既存profileは共通hookの既定動作により従来の予約・拒否を維持する。

関係配列、読み順、Headers、相互参照index、探索stack、分岐状態と候補コピーは、それぞれの実ownerの累積予算から確保前に課金する。関係ownerは実sourceのrecord/spool/output/workと同じ有効上限を継承し、消費済みraw出力creditを復活させない。上限ちょうど/1不足、他owner・変更済み上限の拒否、繰り返しの同一fingerprint、失敗・再試行時の累積消費を実統合で検証した。

最終feature CLI統合は **74 passed、5.71秒**。元の原ノ味とArial Unicodeを有効にし、相互参照、同一branchの複数脚注、未配置参照、仮需要を破棄するbacktrack、次ページの継続、同時配置が1単位不足する場合の拒否を含む。初回の相互参照試験で見つけたJointPageNoFitは上記依存探索で修正した。従来の「新しい定義を開始できず拒否する」book-2試験は、同じ実入力で2定義を完了する検証へ更新し、不足時の拒否は実測上限と1不足の別試験で確認した。

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' \
TYPAXIS_BOOK_STRUCTURE_RELATIONS_PROBE=/private/tmp/typaxis-book-v2-structure-relations-probes-final \
cargo test --manifest-path workspace/Cargo.toml --locked \
  --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --bin typaxis \
  --features book-v2-staging book_v2_resources -- --include-ignored
```

Log: `/private/tmp/typaxis-book-v2-structure-relations-tests-final.log`。上記font/probe変数は試験専用であり、公開CLIには渡さない。

独立検証は `tools/verify_book_v2_structure_relations.py <probe-directory> --self-test` で **30 probes、27 notes、31 reference edges、60 header relations、23 moved notes、4 explicit unplaced notes、95 tamper rejections**。元階層と実BDCの再検証は `tools/verify_book_v2_source_structure.py <probe-directory> --self-test` で **30 probes、1,038 nodes、438 MCID bindings、145 excluded Artifacts、180 tamper rejections**。ともにbundled pypdf Pythonで実行した。Logs: `/private/tmp/typaxis-book-v2-structure-relations-independent-final.log`、`/private/tmp/typaxis-book-v2-structure-relations-source-independent-final.log`。

この段階でも最終StructElem/ParentTree/IDTree、annotation/OBJR/navigation、元page masterを使う完全なglobal object graph/PDF、残るsource/layout形式、正式元全巻exportと公開・管理host・独立・人手による全巻受入は未完である。原フォント・正式元全巻packageは変更していない。

共通paginationの通常feature library回帰は **93 passed、0.18秒**。保存済み正式VMB小規模入力の共通driverは **1 passed、1.36秒**。コマンドは同じlocked manifest/target-dirで、それぞれ `cargo test -p typaxis-pagination --lib` と、`TYPAXIS_COMMON_BODY_JOB=/private/tmp/vmb-legacy-source-integrity-public/typaxis-body-2416664007`・`TYPAXIS_COMMON_BODY_PDF=/private/tmp/typaxis-book-v2-structure-relations-legacy-vmb.pdf` を指定する通常feature CLI `production_common_driver_saved_vmb_job -- --include-ignored`。Logs: `/private/tmp/typaxis-book-v2-structure-relations-pagination-regression.log`、`/private/tmp/typaxis-book-v2-structure-relations-saved-vmb.log`。生成PDFは前段の保存済みPDFとbyte一致した（86,609 bytes、SHA-256 `a1f90f2c6f1de613a92cbe106cfb1561c614e721701959668bcffbc9b643c51d`、`/private/tmp/typaxis-book-v2-structure-relations-saved-vmb-comparison.log`）。この比較は小規模な既存経路の回帰であり、book-2全巻受入ではない。

通常feature CLIのproduction回帰も **249 passed、250.91秒** で終了した。5,000 distinct SVG Formsと5,000 aliasの実配置を両方含み、公開CLIの5試験も試験専用font環境なしで通過した。コマンドは `cargo test --manifest-path workspace/Cargo.toml --locked --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --bin typaxis production_ -- --skip production_common_driver_saved_vmb_job`。保存済み入力を必要とする1試験のみ上記の明示指定で別実行した。Log: `/private/tmp/typaxis-book-v2-structure-relations-legacy-production.log`。これはローカル機能回帰であり、管理hostでの時間/RSS性能受入ではない。今回の最終統合・回帰buildにcompiler warning/errorはなく、全実行はterminal exit 0で終了した。対象Rustファイルのformat、Python検証器の構文、diffの空白検査も成功した。

### 14.154 book-2の実リンク領域・宛先・アウトラインと通常参照文字

`BookV2NavigationGeometryBuilder`を実source構造・脚注関係へ接続した。通常のURI/internal Link、anchor Reference、脚注の往路と最初の描画済み参照への戻り先を保持する。実marked groupの文字・数式・画像・番号の矩形を、元のsource階層を使ってLinkへ帰属させる。Noteの読み順上の移動は本文anchorや参照Linkの描画領域を広げない。各Linkの実ページ・fragmentごとに矩形を作り、ページ別範囲と次の矩形への連鎖を保持する。繰り返しheaderとseparatorのArtifactには注釈を複製しない。

block anchorは最初の意味あるfragmentの実描画領域、inline anchorは元の配置x/baselineを使う。未配置の宛先は座標を捏造せず明示的なNoneとして保持する。実描画済みLinkまたはoutlineが未配置の宛先を指す場合は、その元owner付き`UnplacedDestination`で拒否する。source上で重なるLink（通常Link内の脚注参照を含む）は`ConflictingLinks`で拒否する。outlineは元のlabel・destinationと親・前後の兄弟・最初/最後の子・子孫数を保つ。ここでの座標はY-downで、最終pageの変換、PDF annotation/OBJRやoutline object bytesは後続の接続を必要とする。

通常anchor Referenceには元Referenceの下に生成LinkとSpanを設け、実TextのMCIDをSpanへ結び付ける。§14.153に記載した通常Referenceへの直接bindingはこの形へ更新した。脚注のReference/Link/Lblとは別のslotを使う。実結合試験から見つかった未接続のText-format参照は、元outlineの明示labelを優先し、なければ元headingの通常文字・装飾・改行から表示文字列を生成する。複雑なheadingや非headingで明示labelがない場合は`MissingReferenceLabel`で拒否し、anchor IDや番号の仮文字を作らない。生成文字はCounter namespaceで所有し、PageReferenceの候補文字とは区別する。元のUTF-8と生成provenanceを実フォントの整形、行配置、ActualTextへ渡す。凍結済みの既存profileはこのsource生成へopt inしない。Number-formatの表示と動的なページ参照の全体収束は引き続き未完である。

navigationの全保持配列と一時配列は実sourceの累積record/spool/work予算へ確保前に計上し、outputの既存creditを復活させない。全探索・描画領域の結合・文字列を含むfingerprint計算へworkを課金する。生成参照文字はsourceの保持文字数と脚注・ページ参照を含む同一text予算へ加算し、全文の長さを確認してから確保する。

検証結果は実行完了後に追記する。元page masterと完全なglobal object graph、StructElem/ParentTree/IDTree、annotation/OBJRの実bytes、公開profileと正式元全巻export・全巻受入は未完である。

navigationの最終CLI統合は **76 passed、6.37秒**（`/private/tmp/typaxis-book-v2-navigation-geometry-tests-07.log`）。元原ノ味・Arial Unicodeと§14.153のlocked manifest/target-dirを使用し、probe変数のみ`TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE=/private/tmp/typaxis-book-v2-navigation-geometry-probes-07`へ変更した。生成種別Counterをshaping fingerprintと脚注辺の除外に接続した。複数行fixtureの単語間には元のsoft breakを明示し、切れ目のない長語を自動分割して通してはいない。

独立navigation検証は **33 probes、79 destinations、67 source links、68 rectangles、3 outline entries、1 wrapped link、2 semantic rejections、66 tamper rejections**（`/private/tmp/typaxis-book-v2-navigation-geometry-independent-final.log`）。bundled Pythonで`tools/verify_book_v2_navigation_geometry.py /private/tmp/typaxis-book-v2-navigation-geometry-probes-07 --self-test`を実行した。元階層・実BDCからの再構成に加え、Text-format参照の実ActualText全文を元の明示labelへ照合している。

構文libraryの全試験は **113 passed、1.18秒**（`/private/tmp/typaxis-book-v2-navigation-geometry-syntax-tests-final-02.log`）。同じcargo指定の`-p typaxis-syntax --lib --features book-v2-staging`を使用した。Text-format生成も保持済みsource文字・脚注ラベルと合算し、上限ちょうどと1バイト不足を検証した。plain headingの元soft/hard breakは表示labelでそれぞれ1空白となるため、試験の期待値にも元の改行を含めた。複雑なheadingに明示labelがない場合は正確な元owner付きで拒否する。共有経路の回帰結果は最終PDF接続後の検証と併記する。

### 14.155 book-2の元ページ・完全な構造object graphと検査可能なPDF

`BookV2PdfAssemblyBuilder`は、前節の実navigation ownerからfont/image/抽出fontの実object bytes、実marked本文、source構造と脚注関係をたどり、一つのPDF 1.7を組み立てる。元の単一page master・既定master一致・selection ruleなしを要求し、実測inline ownerのbody rectangleが元のbodyと完全一致することを確認する。複数master、未接続のheader/footer contentやcolumnsを黙って捨てない。これらは`UnsupportedPageMaster`、元のbodyと異なる測定結果は`PageFrameMismatch`で拒否する。

元のwidth/heightをMediaBoxへ、元のtrimをPDF座標へ変換したTrimBoxへ使う。実markedページ数と順序を保ち、各pageの本文全体へ元heightを使うY反転を一度付ける。Text/画像/Formと抽出用fontを実resource辞書へ結び付ける。既存の実object番号を変更せず、それらの使用範囲の後へCatalog、Pages、Info、XMP Metadata、Resources、構造/名前tree、各page/content、StructElem、annotationとoutlineを割り当てる。番号の空きを架空objectで埋めず、classic xrefのfree entryとfree-list連鎖として表現する。

StructElemは元のrole・language・Alt・semantic kind、実関係ownerの読み順、実MCRのpage/MCID、Linkの実OBJRを持つ。生成した一意な固定長IDをIDTreeとtable Headersへ一貫して用い、THのScope、rowspan/colspan、ListNumbering、Reference/Noteの相互Refを保持する。ParentTreeは各ページの実MCID列と、各annotationのStructParentから元Linkへの対応を格納する。ArtifactをParentTreeへ登録しない。

URI/internal/脚注往復Linkは、前節の実fragment矩形をPDF座標へ変換したannotationとなる。URIの元byteをPDF hex-stringとして保持する。内部宛先は実page objectとXYZ座標へ結び、未配置anchorは名前treeへ公開しない。元のUTF-8順序を保つanchor mapからNames/Destsを生成する。outlineの元label、親・兄弟・子・子孫数と実destinationを保持する。InfoとXMPは元metadataを使い、壁時計の時刻を追加しない。既存XMP encoderはフィールドを受け取る共通のstreaming helperへ分け、既存経路の表記を保持した。

全objectの位置表（空き番号を含む）、名前index、元nodeからLinkへのindex、最終PDF bufferは確保前に累積record/spoolへ計上する。文字列escape、二度の実encode、stream長の計数、xref、全PDFのfingerprintへ実workを課金する。既に計上済みのfont/image/marked bytesは、計数が成功した最初の最終buffer予約時に限りoutput creditを置換する。コピー先spoolは全量を課金し、再生成・途中失敗・再試行でcreditを再利用しない。node×Linkの全組合せを走査せず、予約済みのnode indexからOBJR列へ進む。

初回checkの問題（PDF crateからの未依存metadata型の参照、抽出object bytesのaccessor、ページ矩形sliceのindex）は修正した。metadata型は既存syntax境界から再公開し、PDF crateにdocumentへの直接依存は追加していない。最終check成功は`/private/tmp/typaxis-book-v2-pdf-assembly-check-02.log`。初回CLI compileの試験補助内矩形helperも修正し、統合 **76 passed、6.82秒**（`/private/tmp/typaxis-book-v2-pdf-assembly-tests-02.log`）。元原ノ味とArial Unicodeを明示した従来のCLI feature試験で、probeは`TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE=/private/tmp/typaxis-book-v2-pdf-assembly-probes-02`。

独立検証器`tools/verify_book_v2_pdf_assembly.py`は、完全な元wire packageからMediaBox/TrimBox、実本文命令、構造階層と関係、MCR/ParentTree、annotation/OBJR、outline/名前tree、Info/XMPを再構成して実PDFへ照合する。classic xrefのoffsetと空き番号の連鎖も検証する。初回の検証器では元marked-page ownerのq/Q境界が期待列に含まれていなかったため、元encoderに存在する境界を明示した。PDF本文を変更して照合を通したものではない。

現時点で **18 source PDFs、112 pages、1,011 structure nodes、68 annotations、62 tamper rejections** が成功（`/private/tmp/typaxis-book-v2-pdf-assembly-independent-04.log`）。13件は試験側の測定bodyが元masterと異なるための正確なPageFrameMismatchであり、固定MediaBoxの検査envelopeで置き換えていない。MuPDF 1.28.2が全112 pagesを36 dpiで描画し、pdftotextが全18 PDFsをUTF-8抽出した。両toolに診断出力はなく、終了code 0（`/private/tmp/typaxis-book-v2-pdf-assembly-render-extract.log`）。原ノ味CFFを含むPDFは3件あり、日本語とIVSの実抽出も保持している。試験用fontには描画輪郭を持たないfixtureもあり、これらの白いglyphを実出版品質の証拠とはしていない。

追加の元trim・Unicode metadata・未選択masterの試験、共有・既存経路の回帰結果は下記へ追記する。これは検査可能な実PDFであり、`VerifiedPdfBytesReceipt`、公開profile許可やPDF/UAの適合宣言ではない。XMPにもPDF/UA適合フラグをまだ付けない。動的なページ参照の全体収束、Number/複雑なText参照、空段落、再帰/定義table、残る著者kind、複数masterやheader/footer/columns、正式元全巻exportと公開・管理host・独立・人手による全巻受入は未完である。

最終CLI統合は **77 passed、7.37秒**（`/private/tmp/typaxis-book-v2-pdf-assembly-tests-final.log`）。元の非ゼロtrim、Unicode title/author/subject、XML escape、identifier/keywords/created/modifiedを持つ入力、および2個のmasterから元の選択証拠なしに出力しない入力を追加した。初回の追加fixtureはkeywordsの必須正規順序を満たしていなかったため、sourceの順序を修正した。全font/image/抽出objectについて、最終PDF内のbytesが元の実object bytesと完全一致することも確認した。

最終独立検証は **19 source PDFs、113 pages、1,015 structure nodes、68 annotations、122 tamper rejections**（`/private/tmp/typaxis-book-v2-pdf-assembly-independent-final.log`）。拒否した14入力の内訳は、元body不一致13件と、未選択の複数master 1件。Info/XMP全metadata、実MCIDのParentTree差し替え、xrefのoffsetおよびfree-list切断を含む検査も成功した。既に描画・抽出した18 PDFsはbyte一致を確認して証拠を継承し、追加1 PDFのみ新たに両toolで検査した。合計19 PDFs / 113 pagesに診断出力なし（`/private/tmp/typaxis-book-v2-pdf-assembly-render-extract-final.log`）。これは描画機能の検査であり、全巻の人手による読解・組版品質承認ではない。

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' \
TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE=/private/tmp/typaxis-book-v2-pdf-assembly-probes-final \
cargo test --manifest-path workspace/Cargo.toml --locked \
  --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --bin typaxis \
  --features book-v2-staging book_v2_resources -- --include-ignored
/Users/kazuyoshitoshiya/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 \
  tools/verify_book_v2_pdf_assembly.py /private/tmp/typaxis-book-v2-pdf-assembly-probes-final --self-test
```

共有PDF libraryは通常の **88 passed、0.62秒** と、元原ノ味を明示したignored試験 **1 passed、0.98秒**（`/private/tmp/typaxis-book-v2-pdf-assembly-pdf-tests.log`、`/private/tmp/typaxis-book-v2-pdf-assembly-pdf-original-font.log`）。同じcargo manifest/locked/target-dirで`-p typaxis-pdf --lib --features book-v2-staging`、追加1件には`cff_v2_original_pdf_font_objects_and_exact_text -- --include-ignored`を指定した。通常feature CLI production回帰は **249 passed、283.25秒**（`/private/tmp/typaxis-book-v2-pdf-assembly-legacy-production.log`）。両方の5,000 SVG試験と公開CLI試験を含む。コマンドは§14.153の通常feature production回帰と同じで、試験専用font環境は渡していない。

保存済み正式VMB小入力の通常feature共通driverは **1 passed、1.37秒**（`/private/tmp/typaxis-book-v2-pdf-assembly-saved-vmb.log`）。入力・コマンドは§14.153と同じで、出力先は`/private/tmp/typaxis-book-v2-pdf-assembly-legacy-vmb.pdf`。86,609 bytes、SHA-256 `a1f90f2c6f1de613a92cbe106cfb1561c614e721701959668bcffbc9b643c51d`で、前段階のPDFとbyte一致した（`/private/tmp/typaxis-book-v2-pdf-assembly-saved-vmb-comparison.log`）。これらのローカル試験時間は管理hostの性能ゲートを代替しない。元の全巻package・原フォントは変更していない。

最終の統合・共有PDF・通常feature回帰・保存済みVMB試験はすべてterminal exit 0で終了し、compiler warning/errorはない。今回のRust対象ファイルのformat、Python検証器の構文、diffの空白検査も成功した。

### 14.156. 共通PDF生成と実ページ参照の反復（2026-09-09）

`typaxis-pdf::book_v2::BookV2PdfPipeline`を追加した。実際に選択したBodyDisplayから、font選択/closure/program/CID、font objectsと本文命令、raster/vector/image objects、marked content、元構造・関係・navigation、元ページのPDF assemblyを一つの呼び出しで作る。中間ownerをcallback終了まで保持し、すべての工程が成功した場合だけ実PDFを渡す。本文font処理の消費を画像側へ引き継ぎ、工程の失敗時も生成builderが消費したrecord/spool/output/workを保持する。失敗したnavigation/assemblyのcallback未到達と再試行の消費保持も実入力で検査した。

PDF assemblyには、元のPage-format Referenceごとのowner、組版に使った候補ページ番号、リンク先の実ページ番号、参照自身の配置有無を追加した。候補は実source flowの生成文字から、リンク先は実navigationの配置から取得し、全参照の件数・ownerを照合してfingerprintに含める。未配置の参照と未配置のtargetには架空のページ番号を発行しない。描画された参照のtargetが未配置なら既存navigation段階で拒否する。この観測単体は収束の証明ではない。

`typaxis-cli::book_v2_resources::with_converged_book_v2_pdf`は、元のsource-admitted bodyとresource-set /3 ledgerから実際に生成を反復するprivate driverである。元masterのbodyを使い、全Page参照を初期候補1で収集し、候補文字の生成、実fontによる行再整形、表/脚注を含む安定ページ分割、実配置、display、上記共通PDF生成までを毎回実行する。元resource admission・vector binding・native math computationは同じownerを再利用する。数式番号のshapeとvector blockも実際のbody flowに接続する。

Page参照がある場合、実ページと候補文字が一致し、かつ連続する2候補でdisplayと完全なPDFのfingerprintが一致してからcallbackへ渡す。一致しない候補のfingerprintを次の比較証拠には使わない。参照のない本文は、実ページ選択/配置の2回比較を完了した1候補で終了する。元の参照一覧を保ったまま未配置targetの候補だけを保持するため、未配置の脚注を削除したり参照を勝手に本文へ移動したりしない。

`max_layout_passes`と`max_line_reshape_passes`は候補をまたいで共有する。layout側に残り再整形回数を渡すAPIを加え、元limitsのidentityを変更せず次の再整形前に上限を検査する。最初の行分割は再整形回数に含めず、その候補探索workは含める。過去のnative計算/行分割/ページ/PDF work、および完了した候補の下流record/spool/output消費を次の候補へ持ち越す。生成途中のPDFを消費者へ返さず、同じ呼び出し内でエラーを握りつぶして予算をリセットする再試行もしない。source admissionと一時的なsource/shaping storageは既存の段階別上限に従う。全段階の一時allocationまでを単一ledgerで積算したという主張ではない。

前方参照の実試験では、初期候補「1」のリンク先が12ページに配置され、「12」に更新して再組版し、3候補目の一致で終了する。未配置定義内の別参照も元ownerのまま保持する。各候補で2回の行再整形が必要なため、完了には計6回の行再整形・6回のページ選択が必要だった。既定の再整形4回では正確に停止することを検査し、成功試験では明示的に8回を許可した。6回ちょうどは成功し、5回は失敗する。work・record・spool・outputも実測上限ちょうど/1不足を通して検査した。spoolを小さくする試験では、独立したfont subset上限も同じ有効設定へ明示的に下げ、host configとresource policyのidentityを一致させる。

初期の共通pipeline統合77件が成功し、Page参照観測を含む統合は78件成功（9.54秒、`/private/tmp/typaxis-book-v2-page-feedback-tests-01.log`）。動的driverの最終統合は **82 passed、8.08秒**（`/private/tmp/typaxis-book-v2-convergence-tests-04.log`）。前方/未配置参照、参照のない本文、native分数と繰り返しtable header、4種類のvectorと元数式番号、未選択multiple master拒否、上記累積上限を含む。追加fixtureのnode再採番と、縮小したspoolに対するhost configのM4上限を修正した。productionのsource検査や有効上限の条件を緩和していない。

```sh
TYPAXIS_HARANO_FONT=/Users/kazuyoshitoshiya/v/vmb-container/vmb-core/third_party/rendermath/fonts/HaranoAjiMincho-Regular.otf \
TYPAXIS_NUMBER_BIDI_FONT='/System/Library/Fonts/Supplemental/Arial Unicode.ttf' \
TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE=/private/tmp/typaxis-book-v2-convergence-probes-04 \
cargo test --manifest-path workspace/Cargo.toml --locked \
  --target-dir /private/tmp/typaxis-vmb-book-build -p typaxis-cli --bin typaxis \
  --features book-v2-staging book_v2_resources -- --include-ignored
```

独立検証器は、元Page参照のowner/targetと実候補一覧から期待する観測を再構築し、実PDF命令のActualTextを候補文字へ照合する。候補と観測を同時に改変しても実文字との不一致を拒否する。driver callbackから直接保存したPDFも、同じ実displayに対応する元構造・命令probeと結び付けて検査する。これは別の固定ページenvelopeの検証ではない。`tools/verify_book_v2_pdf_assembly.py /private/tmp/typaxis-book-v2-convergence-probes-04 --self-test`をbundled pypdf Pythonで実行した最終結果・共有回帰は下記へ追記する。

先行03 probeの独立検証は **28 source PDFs（うち実driver callback 5件）、179 pages、1,528 structure nodes、97 annotations、14 explicit unsupported inputs、201 tamper rejections** が成功。Log: `/private/tmp/typaxis-book-v2-convergence-independent-03.log`。27個の異なるPDF byte列・167 pagesすべてをMuPDFで36 dpi描画し、pdftotextでUTF-8抽出した。両toolはdiagnosticなし・exit 0（`/private/tmp/typaxis-book-v2-convergence-render-extract.log`）。最終04 probeのPDF集合が03とbyte単位で同一であることも確認した。

前方参照driverの最終観測は、3候補、行再整形6、ページ選択6、records 16,680、spool 673,626、output 137,454、work 289,063。生成PDFは実12ページを参照し、未配置参照は候補1・targetなしのままである。これらはprivate実装のローカル機能証拠であり、公開CLI dispatch、`VerifiedPdfBytesReceipt`、4 manifest、PDF/UA宣言、管理hostの時間/RSSや人手の全巻受入を発行しない。Number/複雑なText参照、空段落、再帰/定義table、残る著者kind、複数master/header/footer/columns、正式元全巻exportと全巻受入は引き続き未完である。§14.155の「動的ページ参照未接続」は本節のprivate driverについて解消したが、公開・全巻完成を意味しない。

最終04 probeでも独立検証は **28 PDFs（実driver 5）、179 pages、1,528 nodes、97 annotations、14明示拒否、201改変拒否**で成功した（`/private/tmp/typaxis-book-v2-convergence-independent-final.log`）。共有PDF回帰は **89 passed、1.13秒**。元原ノ味を明示し、同じmanifest/locked/target-dirで `cargo test -p typaxis-pdf --lib --features book-v2-staging -- --include-ignored` を実行した（`/private/tmp/typaxis-book-v2-convergence-pdf-regression.log`）。元全巻package、元font、公開profileは変更していない。

layout共有回帰は **69 passed、0.35秒**（`/private/tmp/typaxis-book-v2-convergence-layout-regression.log`）。同じmanifest/locked/target-dirで `cargo test -p typaxis-layout --lib --features book-v2-staging` を実行した。最終統合・PDF/layout回帰・独立検証はすべてterminal exit 0で終了し、compiler warning/errorはない。今回の新規driver/pipelineと再整形APIのformat、Python検証器の構文、diffの空白検査も成功した。

### 14.157. 空段落・非描画行の実配置とリンク先（2026-09-09）

Book-2の空段落とアンカーだけの段落を、元のcomputed line-heightを持つ非描画行として配置するようにした。従来の`EmptyParagraph`拒否を、架空の空白文字・glyph・画像で回避しない。`ProductionInlineParagraph::empty`は元ownerと0個の論理unitを保持し、明示呼び出しで1行を選択する。普通のitemizationへ誤って空unitを渡す場合の拒否は維持する。source adapterでBook-2だけが明示的な空行を選び、凍結済み経路の空段落の扱いは変更しない。

空行のcontent ascent/descentは0で、既存のline metrics計算により元行高を前後のleadingへ配分する。固定小数点の半端は従来どおりties-to-evenで扱い、先頭leadingをbaselineとする。本文の前後の空き、字下げ、list/脚注markerの実字形によるleading/trailing拡張は、既存の同じbody item処理を通る。1行分の選択・出力recordと候補workを計上し、0 work/0 line、上限ちょうど、失敗後の再試行を検査する。computed line-heightがない場合に任意の高さを代入しない。

`BookV2AnchorDisplay::nonpainting_lines`は、実選択行とplaced fragmentを照合した非描画行を保持する。空段落だけでなく、連続する明示改行の途中でBreakのみを持つ行も含む。元paragraph owner、flattened fragment index、実line box、元header/繰り返しheaderの役割を束縛し、baseline一致とrecord/work上限を検査する。文字・画像・数式のpaintやMCIDは生成しない。

navigationはこの実line boxを元source nodeと祖先のdestinationに使う。annotation rectangleは従来どおりpaintからのみ作り、空行の横幅を架空のリンク領域にしない。繰り返しheaderの空行は同じsource destinationを上書きしない。空行の後にあるpaintを先頭と誤認せず、逆に文字の後の空行を先頭へ繰り上げないよう、実page/fragment順で最初の配置を選ぶ。inline anchor自身のdestinationは同じ実行のbaselineへ置く。PDF crateへlayout/paginationの直接依存は追加せず、display ownerの型付き観測を利用した。

新しいfixtureは、完全に文字のない1ページ、字下げされたアンカーだけの段落、空段落を含む3段落が2ページに分かれる本文、空のtable headerとその再掲、空の要求済み脚注、連続hard breakの途中の空行を実source-to-PDF driverへ渡す。段落・空行前後のcontainer/inline anchor、空段落へのPage参照、空のPにMCIDが付かないこと、元順序でのdestinationを照合する。初回のfont-free試験で、font streamを持たない再試行では次ownerのreservationを先に消費することが分かり、試験の期待recordをその実処理に合わせた。productionの予算を返却したり、追加のfont streamを生成したりしていない。版面の固定x座標を期待していた試験も、元の字下げを含む実line boxへの照合へ訂正した。

最終CLI統合は **85 passed、8.19秒**（`/private/tmp/typaxis-book-v2-empty-lines-tests-final.log`）。コマンドは§14.156と同じ元原ノ味/Arial Unicode・manifest/locked/target-dir指定で、probe保存先を`TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE=/private/tmp/typaxis-book-v2-empty-lines-probes-final`とした。

独立PDF検証は **36 source PDFs（実driver callback 9件）、213 pages、1,694 structure nodes、107 annotations、14明示拒否、261改変拒否** が成功（`/private/tmp/typaxis-book-v2-empty-lines-independent-final.log`）。独立navigation検証は **43 probes、113 destinations、93 links、91 rectangles、3 outline entries、1 wrapped link、2 semantic rejections、103改変拒否**（`/private/tmp/typaxis-book-v2-empty-lines-navigation-independent.log`）。各検証器へ上記最終probe directoryと`--self-test`を指定し、bundled pypdf Pythonで実行した。非描画行の元ownerと元Break node、line box、最初のdestination、ActualTextとannotation/ParentTree等を別々に検査する。

35種類の異なる実PDF byte列・201 pagesをMuPDF 36 dpiで描画し、pdftotextでUTF-8抽出した。すべてdiagnosticなし・exit 0（`/private/tmp/typaxis-book-v2-empty-lines-render-extract.log`）。文字のないdriver PDF `driver-b3208e595bdce8d8724a01239587fde1791716af173052d9de71e971254b415f.pdf`は、pypdfでも1 page・font 0・XObject 0・抽出文字なしを確認した。試験用fontの空glyphを描画品質の証拠にはしない。

共有linebreak回帰は **46 passed**（`/private/tmp/typaxis-book-v2-empty-lines-linebreak-regression.log`）、元原ノ味を明示した共有PDF回帰は **89 passed、1.21秒**（`/private/tmp/typaxis-book-v2-empty-lines-pdf-regression.log`）。コマンドは同じmanifest/locked/target-dirで、それぞれ `cargo test -p typaxis-linebreak --lib` と `cargo test -p typaxis-pdf --lib --features book-v2-staging -- --include-ignored`。通常featureと保存済みVMB入力の回帰は完了後に追記する。

§14.156までの空段落の未接続は本節のprivate本文経路で解消した。これは空のcontainerに段落を補う変更ではない。Number/複雑なText参照、再帰/定義table、残る著者kind、複数master/header/footer/columns、正式元全巻export、公開CLI/manifest/VerifiedPdfBytesReceiptと全巻の管理host・独立・人手ゲートは引き続き未完である。公開profileとPDF/UA適合宣言は発行していない。

保存済み正式VMB小入力の通常feature共通driverは **1 passed、2.68秒**（`/private/tmp/typaxis-book-v2-empty-lines-saved-vmb.log`）。§14.153と同じ入力・コマンドを用い、出力を`/private/tmp/typaxis-book-v2-empty-lines-legacy-vmb.pdf`へ保存した。86,609 bytes・SHA-256 `a1f90f2c6f1de613a92cbe106cfb1561c614e721701959668bcffbc9b643c51d`で、前段のPDFとbyte一致した（`/private/tmp/typaxis-book-v2-empty-lines-saved-vmb-comparison.log`）。通常feature CLI production回帰では、5,000 distinct SVGと5,000 aliasesを含む従来の実配置試験を実行している。これは管理hostでの時間/RSS受入を代替するものではない。

通常feature CLI production回帰も **249 passed、263.33秒** で終了した（`/private/tmp/typaxis-book-v2-empty-lines-legacy-production.log`）。同じmanifest/locked/target-dirで `cargo test -p typaxis-cli --bin typaxis production_ -- --skip production_common_driver_saved_vmb_job` を実行し、5,000 distinct SVG Formsと5,000 aliasesの両方、公開CLIの共通選択・manifest接続を含む。保存済み入力を必要とする1件は上記で別実行した。最終統合・共有回帰・既存経路・保存済み入力・独立検証はすべてterminal exit 0で終了した。compiler warning/errorはなく、変更した新規空行/描画/navigation/testファイルのformat、Python構文、diffの空白検査も成功した。元全巻package・元fontは変更していない。

### 14.158. 記述リストの版別carrierと用語の所有権（2026-09-09）

元全巻で使われる30件のdescription listに対し、[ADR-0040](../adr/ADR-0040-book-2-description-lists.md)で非公開1.5の専用`description_list`を定めた。list/item/termを別nodeとして保持し、termのinline childrenと説明のblocksを分離する。用語内の装飾・数式・ベクター・リンクは既存のtyped inlineのまま保持する。`ordered`、`start`、生成marker、termの`kind`、未知memberは受理しない。item列・用語children・説明blocksは空配列を拒否し、再帰的な実内容の妥当性はsyntax側で別途確認する。

共有carrierはtestまたは明示的`book-v2-staging`時だけ専用variantを含み、sealed kindの版境界でdescription itemsのserialize/deserializeを制限する。1.4はfeature付きでも専用variantを拒否し、typed legacy DTOから空itemsを直接構築してもencodeできない。既存の通常listの型・canonical bytesと公開1.4入口は維持する。用語を含む一時的な1.3 supporting-field検証viewは保持・出力せず、返すwire/JCSは専用kindと元の所有者を維持する。ASTはitemとtermを各1nodeとして数え、term childrenと説明blocksを本来の深さで検査する。

この段階で追加するのはcarrierと境界検査であり、記述リストの組版成功ではない。source loweringは専用`DescriptionListStaging`エラーで止める。言語・flow・structureも対応済みリストへ読み替えて通過させない。用語の元source span、内部link、参照、数式speechをたどる共有visitorにはtermを追加するが、それ自体を配置の証明としない。次にtyped domain、term/body style、実行分割・ページ選択、`L/LI/Lbl/LBody`と実MCIDを接続する必要がある。正式exporter、公開CLI、元全巻、managed-host、独立PDF・人手受入の未完了条件は変わらない。

検証はdocument-package全体 **57 passed（0.44秒）**、共有syntax **114 passed（1.40秒）**、既存Book-2統合 **85 passed（8.37秒）** で終了した。新規6件を含む13件のsuccessor carrier testは、全再帰slotの往復、必須/未知/重複/null/空要素、1.4のraw/typed拒否、term内native/vector検証、typed変更後の再検証、headerへの不正挿入、metadata/outlineの既存2nodeを含む14/13node・6/5depthの境界を確認する。既存の実PDF **36件（うち実driver callback 9件）、213ページ** は独立検証を通過し、§14.157の最終PDFと全件byte一致した。未対応入力14件・改変拒否261件も再確認した。全最終実行はterminal exit 0。これは記述リスト自体の組版受入ではなく、carrier追加と既存PDF互換性の証拠である。コマンド・ログ・比較hashは[進捗記録](28-vmb-book-production-progress.md#book-2-description-list-carrier-and-term-owners-design-14158)に保存した。

### 14.159. 記述リストのtyped domainと用語内数式style（2026-09-09）

§14.158のcarrierを受け、非公開Book-2の構文準備は`SemanticBlock::DescriptionList`、`SemanticDescriptionItem`、`SemanticDescriptionTerm`を保持する。termはparagraphへ変換せず、独立したcommon（node/span/classes）と実内容判定、inline vector所有者を持つ。元inline treeはwireに、native mathの構文木・source/speechは既存のBook-2 math側に保持する。native mathとvectorのownerはterm nodeであり、listやitemへ付け替えない。

検証順序はlist→item→term→term inline→definition blocksである。dense preorder、list/item/term/inlineのsource所有範囲、item列とterm/definitionの元位置順序、term classesを検査する。termに実内容がない場合（空Textやbreakのみ）や空の説明配列を拒否する。空行を含む説明ブロック自体は既存の段落規則に従い、termやitemを黙って削除しない。一般再帰slot（別description、通常list、semantic container、table head/body、figure/vector caption、footnote）でも専用kindを保持する。

Book-2だけが`description_list`と`description_term`のstyle selectorを受理する。明示的extendsを残した専用property sheetで継承・優先度を解決し、通常paragraph/listのselectorを暗黙に適用しない。用語内native mathはterm側のfont設定を受け、説明側のmath/containerへtermの指定が漏れない。数式を再parseしていないことも元sourceの所有ポインタで検証する。旧style入口はfeature付きでもdescription selectorを拒否し、1.5 supporting-field検証用の一時viewだけがselectorを既存property文法へ写す。返すwire/JCSのselectorやdomain kindは変更しない。

Book-2の構文・style準備からdescriptionの一律拒否を外したが、言語/navigation registry、実際のterm/body flow、行・ページ選択、PDFの`Lbl`/`LBody`への接続はまだ別段階である。legacyのprofile/layout/PDF処理に新domainを通して成功扱いにすることはない。元vector/figureを調べる読取visitorは用語や説明を保持し、legacyの配置・出力入口は不一致を拒否する。CLIの非公開Book-2 featureからmanifest側の型整合用featureを伝播するが、successor manifestや公開profileの発行は追加していない。正式exporter・元全巻・公開CLI・管理host・独立PDF/人手受入の条件は引き続き未完了である。

この段階の最終検証は、共有syntax **118 passed（1.28秒）**、document-package **57 passed（0.45秒）**、Book-2統合 **85 passed（8.86秒）**、通常featureのCLI production **249 passed（259.47秒）**。通常CLIでは5,000 distinct SVGと5,000 aliasesの配置を再実行した。保存済みVMB jobを必要とする1件は今回は除外し、前段の証跡を維持している。`--workspace --all-features`のcargo checkも成功した。実PDF36件・213ページの独立検証と改変拒否261件が通り、全36件は§14.158のPDFとbyte一致した。これらはdescription domain/styleと既存経路互換性の証拠であり、descriptionを実際に配置したPDFの受入ではない。コマンド・ログ・hash一覧は[進捗記録](28-vmb-book-production-progress.md#book-2-description-domain-and-term-math-styles-design-14159)を参照。

### 14.160. 記述リストの言語・本文flowと著者用語の構造（2026-09-09）

Book-2の言語所有者を専用の`BookV2LanguageNodeKind`に分け、`DescriptionList`、`DescriptionItem`、`DescriptionTerm`を追加した。共有visitorは版に対応する型のlanguage siteを収集する。用語はitemから、用語内inlineはtermから、説明blocksはitemから言語を継承し、それぞれの明示言語を正規化する。用語の指定を説明へ漏らさず、native/vectorの元所有者も保持する。旧`StagingLanguageNodeKind`と凍結済み言語registryへの変換・新variantは追加していない。

source flowは専用のlist/item metadataと`Begin DescriptionList → Begin DescriptionItem → Begin DescriptionTerm → 用語inline → End term → 説明blocks → End item → End list`を保持する。用語のinline-bearing storageは見出し等と同じ組版primitiveを使うが、source kind、owner、span、style、eventは専用のままである。通常list registryに登録せず、箇条書きmarkerを生成しない。文字は元text bufferを借用し、数式・vector・link・referenceは既存のtyped inlineとして後段へ渡す。list/item metadataの合計は`max_fragments`、元nodeとinlineはAST上限、正規化言語と生成参照は元の共有text上限で制限する。再検証は元body/navigationの所有者と再構築した専用metadataを照合する。

独立したdescription用property sheetからlist/term styleとpage指定を取得する。listの字下げは実frameを狭め、term自身の字下げはその内側に適用する。説明のfont等はlistの指定を継承し、termの指定を継承しない。前後の空きとkeep指定は実本文のページ選択へ渡す。任意のmarker幅・gapは足さない。未対応のnamed pageは従来と同様に明示拒否する。

source構造は`L / LI / Lbl / LBody`となり、`Lbl`は元termの`Source` slotである。用語の実inline構造はその子、説明の実block構造はitemに対応する`ListBody` slotの子となる。生成`ListLabel`へ用語を置き換えない。実MCIDは元text/math/image/参照の描画に対応し、用語の親`Lbl`へ架空の描画を追加しない。独立検証器もwireのtermを再帰し、専用role・親子関係・元言語を再構築する。

検証結果とコマンドは[進捗記録](28-vmb-book-production-progress.md#book-2-description-language-flow-and-source-structure-design-14160)へ記録する。これは非公開Book-2の接続であり、正式元全巻export、公開CLI・4 manifest・`VerifiedPdfBytesReceipt`、全巻の独立/PDF-UA・管理host・人手受入の完成を示さない。用語に複雑な数式/vectorを含む実PDF、各再帰配置の実PDF、残る著者kind等についても、必要な受入証拠を別に揃える。

本節の最終Rust検証は、Book-2 CLI **87件**、syntax **123件**、layout **69件**、pagination **94件**、通常CLI **249件**、保存済み正式VMB小入力 **1件**の計 **623 passed**。全featureのworkspace checkも成功した。独立PDF検証は **40 PDFs（実driver 11）、235 pages、1,730 structure nodes、143 annotations、14明示拒否、289改変拒否**で成功した。新しいLatin/Japaneseの用語PDF4件・22ページはMuPDF 72 dpi描画とUTF-8抽出を通過し、各PDFの用語12回・説明1回を保持する。日本語の実driver PDFは原ノ味を埋め込んだ4ページで、先頭・最終ページも目視確認した。旧36 PDFsと保存済みVMB PDFはそれぞれ前段とbyte一致している。通常CLIは両方の5,000 SVG試験を含む。全最終実行はterminal exit 0であり、本節の実装・互換性証拠と、上記の全巻・公開受入の未完了条件を区別する。

### 14.161. 数式・ベクターを含む記述用語の再帰配置と脚注（2026-09-09）

用語の実inlineにnative分数、inline vector、math vectorを併記した入力を、本文直下、semantic container、記述リストの説明内、通常list、table head/body、raster/vector figure caption、参照される脚注定義、未参照の脚注定義へ配置する。さらに用語自体へPage参照と脚注参照を加えた入力を含め、11配置を共通の実PDF driverで検証する。用語の元owner・専用20 pt style・source `Lbl`、native/vector terminal、反復headerの追加描画、未参照定義の明示的な未配置を保持する。

この検証で2箇所の接続漏れを修正した。body collectorのdescription metadata cursorの終端照合は、脚注定義の開始時ではなく全source eventの収集後に行う。脚注内にも記述リストを認め、未参照定義も収集の閉包に含める。脚注の読み順の挿入先は、段落・見出しに加え、著者用語のsource `Lbl`を認める。`Source`以外の生成ラベルslotは対象にせず、元source構造やMCIDを変えずにNoteの読み順だけを参照inline branchの直後へ接続する。独立検証器も元termを辿ってこの親子関係を照合する。

Book-2 CLI **88 passed**、pagination **94 passed**。独立PDF検証は **62 PDFs（実driver 22）、261 pages、2,186 structure nodes、153 annotations、14明示拒否、435改変拒否**、脚注・表構造検証は **56 probes、45 notes、48 reference edges、92 header relations、36 moved notes、9 unplaced notes、185改変拒否**で成功した。前節の40 PDFsはbyte一致し、新規22 PDFs・26ページはMuPDF 72 dpi描画とUTF-8抽出に成功した。今回のnative用合成MATH fontの字形は検証用矩形であり、この描画確認を実書籍の文字品質や原ノ味native mathの証明とは扱わない。正式exporter、全巻・公開受入の残件は引き続き[台帳](28-vmb-book-production-progress.md)で追跡する。

### 14.162. VMBの記述リスト実入力と数式ActualText抽出（2026-09-09）

VMBの非公開`stageBookV2Package`から、元RenderBookのdescription itemを`description_list / item / term`として出力する。元inline・数式・参照をterm内に保持し、termを先に、説明blocksを後にsource projectionへ登録する。用語は独自のdense node ID・span・JSON Pointerを持ち、生成番号や段落へ置換しない。空item集合・空term・break/wrapperのみのterm・空definitionを拒否する。用語と説明の間のprojection separatorは用語spanの外へ置き、共有node上限とキャンセル、source/math sidecarの照合を従来と同じ所有者で行う。

公開VMB関数は従来の1.4/profile-1分岐を呼び、非公開1.5経路だけが専用description styleと明示されたprofile-2のCID CFF snapshotを出力する。元フォントbytes/hashを保持し、CID/FD、glyph、権利等の正式admissionはTypaxisが行う。CFF2・可変font・CFF collection等の拒否は維持する。新しいstagingの戻り値は別の非公開型であり、既存public runner・capabilities・config・manifestをprofile-2へ切り替えてはいない。

VMBの実math adapterと元原ノ味から生成した小入力を、書換えずにTypaxisの`HostBookV2InputSession → prepare_admitted_book_v2_body → prepare_book_v2_resources → with_converged_book_v2_pdf`へ接続した。1 description、2 shared SVG resources、3数式配置と日本語本文を含む1ページPDFを生成できた。これは正式コード経路の小規模結合fixtureであり、元全巻30 description・8,149数式配置を完成させた証拠ではない。

この実PDFの独立抽出で、Formula/Figure BDC自身のActualTextをPopplerが使用せず、抽出用glyphのU+FFFCが出る問題を確認した。Book-2の外側は元Formula/Figure・MCIDを保持し、内側にActualTextのみを持つSpanを1つ置く。Artifactは追加Spanを持たず、旧PDF経路は変更しない。追加した開始・終了命令を全て予約・出力・workへ計上し、置換前byteのcreditも実際のscopeごとに照合する。構造上のMCIDを増やしたり、数式のAlt/ActualTextを二重追加したりしない。

`tools/verify_vmb_description_pdf.py`は保存された元packageの本文・用語・math ActualTextから期待文字列を独立に作り、Popplerの`-raw`抽出と比較する。物理改行/改ページ以外の空白・文字・日本語句読点を変更せず、全3数式を元順序で検証する。修正前PDFを拒否し、修正後PDFを受理した。前後の100 dpi RGB描画は完全一致する。最終Book-2 CLI **89 passed**、独立PDF **64 PDFs / 263 pages / 447改変拒否**、scope **57 probes / 965 scopes / 965改変拒否**、marked本文 **57 probes / 208 pages / 52画素一致比較 / 188改変拒否**。VMB exporter全体の`go test ./internal/rendertypaxis -count=1`も成功した。詳細は両リポジトリの検証台帳へ記録する。

### 14.163. VMBの7種の注記と演習に結び付く解答（2026-09-09）

VMBの非公開Book-2 exporterは、`example / counterexample / remark / note / warning / commonError / formalizationNote`を専用semantic kindのまま出力する。後者2種類のwire名は`common_error / formalization_note`である。元Title・Blocks・Tags・anchor・source originを保持し、見出しprefixとseparatorは明示されたpresentationから取得する。本文をresultへ置換せず、記述リストと元数式を含む既存の再帰block経路へ接続する。未対応の非ゼロfieldと見出しpresentation欠落は従来どおり拒否する。

`solution`も独立kindとして元Blocksを保持する。必須の`Solves`は、走査を終えた実exercise owner集合と照合し、後方にある演習への参照も許可する。存在しないtarget、別kind、自分自身、空targetはsource診断で拒否する。明示された解答見出しを演習への内部linkにし、PDFの実destinationとlink rectangleへ接続する。verificationは解答本文の後、formal表示はその後に置く。既存の5 verification表示と3 formal modeを利用し、formal sealのownerは`solution`のままである。元NPA resource・行source・formal recordを保持し、resultへの付け替えは拒否する。

元タグに生成classを追加するとclass順が非canonicalになる問題を、形式解答の実入力で検出した。Book-2 semantic containerだけでコピー済みclass列をソートし、著者タグを削除せずに正式syntaxへ渡す。公開1.4 exporter、runner/config/capabilitiesと既存profileの受理範囲は広げていない。

実math adapterと元原ノ味による、7種の入れ子注記、演習より前に置いた解答、元NPA付き形式解答の3入力を保存し、そのpackage・source・fontを書き換えずにTypaxisの共通Book-2 driverへ渡した。各1ページPDFが生成され、独立した構造・annotation検査とPopplerによる全文抽出を通過した。通常解答は1 descriptionと3数式、形式解答は元NPAと2数式を含む。期待文字列は元packageから作り、物理改行/改ページだけを除外して比較する。各実ページをMuPDF 100 dpiで描画し、日本語・数式・NPA本文の配置も確認した。

最終回帰のコマンド・件数・hashは[進捗記録](28-vmb-book-production-progress.md#book-2-vmb-admonitions-and-solutions-design-14163)へ記録する。これは元全巻の328注記・260解答の全件受入ではない。残るexercise parts/choices/hints、result assumptions、media、番号参照、脚注内table、複数master、公開Book-2 CLI/manifest/receiptと元全巻・管理host・人手の受入は引き続き未完了である。

本節の最終検証はBook-2 CLI **92 passed（11.70秒）**、独立PDF **70 PDFs / 269 pages / 487改変拒否**、修正後のVMB exporter全体回帰 **成功（185.801秒）**。前節の64 PDFはすべてbyte一致した。全実行はterminal exit 0で終了し、変更箇所のformat・Python構文・両repoのdiff空白検査も成功した。全巻・公開受入の未完了条件は上記のとおり維持する。

### 14.164. 演習の小問・選択肢・ヒントと解答参照（2026-09-09）

[ADR-0039](../adr/ADR-0039-book-2-semantic-vocabulary.md)の非公開Book-2語彙へ`exercise_part / choice / hint`を追加した。元IDと本文を持つ各unitを独立したsemantic containerとして保持し、既存exercise/noteや匿名段落へ置換しない。wire、document domain、style kind、syntax変換と実flow/構造は同じ専用kindを保持する。標準PDF roleは`Sect`であり、表示文字やclassから意味種別・番号を推定しない。明示list表示との組み合わせは可能だが、本実装はcallerが解決したunit見出しを使用し、暗黙のlist markerを付けない。公開1.4の3種と公開dispatchは変更しない。

VMBの非公開exporterは、演習内でprompt→parts→choices→hints→任意のsolution link→verification→formalの順序を保持する。小問のTitle・Prompt、選択肢のContent、ヒントのBlocksを既存の再帰block/inline経路へ渡す。unitの元VMB ID、親source range、元のmember/index pointerをsource projectionへ記録する。正確な行番号がないunitに架空の行範囲を補わない。小問のanswer type・formal result参照、ヒントのlevelは、元ownerに束縛したannotationとして保存する。formal result IDだけから証明済み表示やNPA本文を生成しない。

unitの見出し・タイトルseparator、演習から解答へのlink labelは明示settingsを要求する。ヒントのlevelは元schemaの1..9を検査し、見出しpresentationのlevelとも一致させる。小問のanswer typeは元schemaの7値に限る。空body、未知の非ゼロfield、空/重複ID、表示欠落、不正level/typeを拒否する。annotationの件数・UTF-8 byte予算はformal参照とhint recordも含み、source/owner/hash照合によって後からの改変やkind付け替えを拒否する。

演習のSolutionTargetは全走査後に実solutionを探し、そのSolvesが当該exerciseであることまで確認する。未定義、別kind、自分自身、別演習の解答を拒否する。前方・後方の配置順に依存せず、既存のsolution→exercise linkと合わせて実PDFの双方向destination/rectangleへ接続する。元RenderBookが持たない非表示の解答やヒントは追加しない。

実math adapterと元原ノ味から、小問題名の装飾と数式、記述リスト、数式を含む選択肢、level 2のヒント、別に置いた解答を持つ小規模入力を保存した。元package/source/fontを書き換えずに共通Book-2 driverで1ページPDFを生成し、1 description・4数式と日本語本文を元順序で抽出できた。独立PDF構造と両方向のlinkも確認し、MuPDF 100 dpi描画を目視確認した。

検証はVMB focused **4 tests（13負例subcases等を含む）、10.558秒**、全exporter回帰 **成功、176.761秒**、document-package **57 passed**、syntax **123 passed**、Book-2 CLI **93 passed**。独立検証は **72 PDFs（実driver 27）、271 pages、2,608 nodes、161 annotations、14明示拒否、501改変拒否**で成功した。従来70 PDFはbyte一致する。元全巻での各unit全件受入・result assumptions・media・番号参照・脚注内table・高度なpage master・公開CLI/manifest/receipt・speech補完・管理host/独立PDF-UA/人手受入は引き続き残る。詳細は[進捗記録](28-vmb-book-production-progress.md#book-2-exercise-teaching-units-design-14164)を参照。

全featureのworkspace `cargo check`も **成功（1分50秒）**。全プロセスはterminal exit 0で終了した。compiler warning/errorなし、変更ファイルのformat・Python構文・両repoのdiff空白検査も成功し、元全巻packageと原ノ味fontのhashは不変である。

### 14.165. 定理等の仮定文と引用・出典の元inline保持（2026-09-09）

Book-2の閉じた語彙へ`assumption`を追加し、VMB resultの各Assumptions unitを独立したsource groupで保持する。生成したdense node IDは組版ownerであり、元にないVMB anchorは追加しない（wireのanchor_idはnull）。元resultのsource range・VMB ownerと`assumptionUnitIds/index`の参照位置を引き継ぎ、各unitのinlineを元順序のままstatementより前に置く。仮定のラベル、番号、数式の意味文を推測して補わない。wire/domain/style/syntaxの専用kindは実flowとPDF構造まで維持し、標準roleは`Sect`とする。

既存`quote`種別をVMBの非公開exporterへ接続した。引用本文Textを先に、存在するAttributionを後に、それぞれのrich inlineとsourceを保持して出力する。quote block自身の元IDがanchorであり、標準PDF roleは`BlockQuote`となる。見出しprefix、引用符、出典用のダッシュ等は自動追加しない。任意の出典がない場合に空段落を補わず、明示された本文/出典に実内容がない場合は拒否する。

引用と仮定は共通のbounded inline loweringを使用し、装飾・元数式・内部/外部link・脚注等を文字列へ平坦化しない。空配列、break/wrapperだけの内容、未対応の非ゼロfield、引用の空/重複ID、node予算超過はsource診断で拒否する。sourceや表示文字列を変更して空内容を修復しない。container class構築は従来と同じ関数へまとめ、Book-2だけのsort、著者タグの保持、公開1.4の受理範囲を維持する。

検証のコマンド・入力/PDF hash・実抽出と構造の結果は[進捗記録](28-vmb-book-production-progress.md#book-2-assumptions-and-quotations-design-14165)へ記録する。これは全RenderBook形式や元全巻の完成ではない。code/resource本文、media、図表の残る著者field、番号参照、脚注内table、高度なpage master、公開CLI/manifest/receipt、元全巻speechと管理host/独立PDF-UA/人手受入は引き続き必要である。

本節のVMB focusedは **4 tests（7負例subcases等を含む）、23.688秒**、全exporter回帰も **成功（238.692秒）**。Typaxisはdocument-package **57 passed**、syntax **123 passed**、Book-2統合 **94 passed**。独立検証は **74 PDFs（実driver 28）、273 pages、2,700 nodes、165 annotations、14明示拒否、515改変拒否**で成功し、従来72 PDFはすべてbyte一致した。元原ノ味による1ページPDFでは、仮定・引用・出典と6つの意味数式、内部linkを元順序のまま確認した。これは本節の小規模結合受入であり、上記の全巻・公開ゲートとは区別する。

全featureのworkspace `cargo check`も **成功（8分21秒）**。継続中の同じ実行を追跡し、全プロセスはterminal exit 0で終了した。compiler warning/errorなし、変更ファイルのformat・Python構文・両repoのdiff空白検査、元全巻package/原ノ味fontの不変hashも確認した。

### 14.166. 表示元の範囲に結び付く番号参照（2026-09-09）

[ADR-0041](../adr/ADR-0041-book-2-source-number-bindings.md)に従い、非公開1.5のdocumentへ任意の`number_bindings`を追加する。各要素はanchor、番号を所有する元block/group、実際のTextまたはequation-number leaf、その中の非空UTF-8範囲を指定する。例として`定理1.12`や`(1.12)`のうち`1.12`だけを参照できる。表示ラベルから番号を推測せず、別途指定した自由文字列を番号として採用しない。

構文段階で元ownerの子孫関係、TextSpanの包含、UTF-8境界、番号を表示するleaf、anchorの重複と位置を検証する。既存の図caption等のanchorは元位置を維持し、まだanchorがない式等では明示bindingがownerへのdestinationを宣言する。文書metadataなのでsource node/PDF structure nodeは追加しない。collectionと各bindingを共有AST予算へ計上する。

参照は`format: number`と元targetを維持したまま、Counter名前空間へ参照ownerごとの番号文字列を生成する。Text参照と同じ文字数・buffer・fragment予算を共有し、参照側のstyle/languageを通して共通組版へ渡す。bindingがなければ、見出し/outlineが存在してもNumber参照を拒否する。Page候補・収束処理、旧1.4のtyped/raw形式と公開dispatchは変更しない。

VMBではAssumptionsの各inlineを参照解決の走査へ追加した。これまで本文・title等には適用していた解決処理が仮定文では抜けており、未解決のcrossReferenceが残り得たためである。番号の元targetへの解決と、存在しないtargetを仮定のowner位置で診断する回帰試験を加える。

コマンドと結果は[進捗記録](28-vmb-book-production-progress.md#book-2-source-number-bindings-design-14166)へ記録する。VMBの番号表示設定からbindingを生成する正式接続と元全巻・公開ゲートは別途必要であり、このcarrier追加だけで完了とは扱わない。

document-package **60 passed**、syntax **128 passed**、Book-2統合 **97 passed（20.22秒）**。独立検証は **80 PDFs（実driver 31）、285 pages、2,776 nodes、175 annotations、14明示拒否、557改変拒否**で成功し、従来74 PDFはすべてbyte一致した。元原ノ味による2ページの実寸fixtureで、前方/後方Number参照と`定理1.12`の表示・正確な抽出を確認した。最初の極小用紙fixtureは外部抽出に失敗したため受入証拠に採用せず、実寸を明示した入力と再実行結果を台帳へ記録した。VMBのrender/exporter回帰も両方成功している。

全featureのworkspace `cargo check`も **成功（2分13秒）**。最終compilerログにwarning/errorなし。各検証を同じ実行のterminal結果まで追跡し、形式・Python構文・両repoのdiff空白検査、元全巻package/原ノ味fontの不変hashも確認した。

### 14.167. VMBの章・囲み・図・式からの番号binding出力（2026-09-09）

VMBの非公開Book-2 exporterへ`BodySettings.NumberRanges`を接続した。設定は元LogicalNumberと、既存の表示prefixまたはequation-number Text内の明示UTF-8範囲を持つ。章見出し、番号付きのsemantic container、図caption、式番号の実Text leafに対応付ける。番号の値や範囲をprefixから検索・推測しない。元番号との不一致、空/不正/範囲外のUTF-8、未出力targetの設定、bindingの不足と重複を拒否する。

`numberOnly`と`labelAndNumber`は元のUI templateを使用し、番号部分を`format: number`のReferenceとして出力する。参照先の元kindとLogicalNumberを照合する。labelAndNumberのrich labelは同じtargetへのLinkを保ち、Number自身は独立したReferenceとするため、Linkの入れ子を作らない。参照位置へ番号のTextを複写せず、元のゼロ長source span・language・targetを保持する。参照や表示内容の書き換えを、エンコード時に保存済みの内部記録と再照合する。

番号bindingはdocument metadataとして共有node予算へ計上し、source projectionへ偽のnode/textを加えない。sidecarには元kind・LogicalNumber・実owner/label・選択範囲・表示内容のhash・元位置を保持する。式ではRenderBlockの一般位置より精密なMathSourceの位置を、実projection ownerから引き継ぐ。旧1.4のlowererは新設定を拒否し、番号がない既存出力の形式を維持する。

実結合で判明した式番号の`MissingTextStyle`もproducer側で修正した。実際に番号付きの式を持つBook-2入力に限り、指定された本文font family・font size・line heightを`math_vector_block`へ明示する。式番号を省略して成功させたり、Typaxisが未指定フォントを補ったりしない。

[進捗記録](28-vmb-book-production-progress.md#vmb-number-reference-export-design-14167)へ入力・PDFのhashとローカル検証結果を記録する。これは元数式・原ノ味を用いた合成RenderBook fixtureの非公開結合であり、正式公開runner/config、全巻presentation設定の生成と元全巻の受入を完了したものではない。

最終VMB全回帰はrender **35.507秒**、rendertypaxis **240.158秒**で成功した。最終コードの保存入力12ファイルは実PDF検証済み入力とすべてbyte一致し、その保存入力に対する独立抽出・6改変拒否も成功した。Book-2統合 **98 passed**、独立 **82 PDFs / 571改変拒否**、従来80 PDFのbyte一致を確認した。全実行が終了し、形式・構文・diff空白検査と元全巻package/原ノ味fontのhash不変性も確認済みである。

### 14.168. 図タイトルと説明キャプションの両方の保持（2026-09-09）

VMBの非公開Book-2 exporterは、図のTitleを元のrich inlineとして受理する。Titleがある図では、FigureのCaption配列の先頭段落へ明示prefixとTitleを置き、Captionが併記されていれば次の独立段落へ保持する。説明文でタイトルを置換したり、両者を一つの文字列へ平坦化したりしない。タイトル段落は`vmb-figure-title`、説明段落は既存`vmb-figure-caption`のclassを持ち、どちらも標準PDFのCaption配下のPとなる。

明示番号prefixとanchorは先頭段落に一度だけ置く。Number bindingはその実prefix Textの範囲を維持する。タイトルだけの図も実内容とanchorを持つため、他のタイトル内等からの参照先になれる。タイトルがない図の既存wire形状は維持し、旧1.4ではTitleを引き続き拒否する。数式・装飾・language・リンクは共通inline経路へ渡し、元source ownerを保持する。Title/Captionとして指定されたbreakや空wrapperだけの内容は、生成番号の有無にかかわらず拒否する。

図版と両段落の配置・keepは既存の共通Figure flowに委ねる。用紙寸法や画像幅を変更して収める処理、タイトル用の疑似画像や独自座標配置は追加しない。元数式・元原ノ味を使う小規模fixtureの検証結果は[進捗記録](28-vmb-book-production-progress.md#vmb-figure-title-and-caption-export-design-14168)へ記録する。表のTitle/Caption/番号と、全巻・公開ゲートは引き続き未完了である。

VMB全回帰はrender **33.995秒**、rendertypaxis **227.527秒**で成功し、Book-2統合 **99 passed（25.81秒）**。元原ノ味による実PDFで2つのタイトル、独立した説明、7数式と参照を抽出・描画確認した。独立検証は **84 PDFs / 289 pages / 3,076 nodes / 213 annotations / 585改変拒否**で成功し、従来82 PDFと最終producerの入力12ファイルはそれぞれ検証済みのbytesと一致する。全実行が終了し、形式・構文・diff空白検査と元全巻package/原ノ味fontの不変hashも確認した。

### 14.169. 表キャプションの非公開入力契約（2026-09-09）

[ADR-0042](../adr/ADR-0042-book-2-table-captions.md)に従い、Book-2のtableへ任意の`caption`を追加する。値は元のblockを保持する非空配列であり、省略時の表は従来と同じになる。明示null・空配列・未知fieldは拒否する。元のnode ID・span・class・language・rich inline・数式をcanonical往復で保持し、captionは表のhead/bodyより前の独立した子領域として扱う契約とする。

旧1.4はtyped decode/encodeでもcaptionの指定を拒否する。元の凍結carrierによる互換検査だけは、captionを当該表の直前へ置く一時的な検査用の形を使う。保持するwireとcanonical出力にこの平坦化を使わない。captionの実block/inlineは表の一段下から共有AST node/depth予算へ計上し、再帰する数式source version・vector field検査とtyped再エンコードにも含める。

本節の実装時点は入力形式の段階であり、表captionの組版・PDF対応完了ではない（source接続と拒否境界の移動は後続§14.170を参照）。この時点の元sourceのloweringはcaption付きtableを`TableCaptionStaging`、元表ノード付き`L5100`で明示拒否する。元の内容を捨てたreceipt/PDFを作らない。次の接続ではsource所有関係、style/language、実幅・高さ、captionから表への継続、Table/Caption/THead/TBody構造、VMBのTitle/Caption/番号を実装・検証する。検証結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-caption-carrier-design-14169)へ記録する。

入力形式66 tests、syntax 129 tests、全feature workspace check（3分28秒）が成功した。既存Book-2統合99 testsと独立84 PDFs / 585改変拒否も成功し、従来84 PDFはbyte一致する。これは既存対応範囲の回帰検証であり、caption付きtableのPDF成功とは扱わない。全検証実行は終了し、新規Rustファイルの形式・両repoのdiff空白、元全巻package/原ノ味fontの不変hashも確認した。

### 14.170. 表キャプションのsource・本文フロー接続（2026-09-10）

Book-2のtable captionをtyped documentの直接の子blockとして保持する。表のhead/bodyより前に元node ID・span・本文をloweringし、表の所有範囲・source順序・共有node/depth予算を検証する。元ファイルに対するUTF-8境界検査、styleと言語の継承、anchor・Page/Number・脚注参照の走査、表示番号leafの子孫関係にもcaptionを含める。新しいsource nodeや番号文字列を追加して元内容を置換しない。

共通source flowは、元tableにcaptionのevent範囲を保持する。このmetadataも共有table予算に計上し、flow再検証で範囲の削除・改変を拒否する。caption内に別tableがあっても外側tableの範囲を上書きしない。source構造はTable直下にCaption、その後にTHead/TBodyを置き、caption内の通常段落にcell属性を付けない。実際のセルは従来のrow/column/spanとTH/TDの関連を保持する。

本節ではsource loweringの暫定拒否を、共通frame preparationの`PendingTableCaption`へ移した（後続§14.171で計測接続とsearch段階への移動を記録する）。実フォントによる文字整形は可能だが、captionを含む高さ・ページ分割・継続・keepを実装するまでは配置を認可しない。現行の表組版は並列セルの計測を前提とするため、captionを無視したPDFや疑似セルへの変換を受理しない。VMBの表Title/Caption/Number出力、実PDF検証、脚注内table、公開経路と元全巻の受入は引き続き必要である。

Book-2を無効にしたビルド検査で、前段carrierのserde境界指定がfeature/test時だけ有効だった問題を検出した。captionのversion検査は全ビルドで必要なため、再帰するwire型の`WireSemanticKind`境界を常時指定する。意味種別のDefault実装を補ったり、旧1.4でcaptionを許可したりしない。検証コマンドと最終結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-caption-source-flow-design-14170)へ記録する。

最終検証は入力形式66 tests・syntax 134 tests、Book-2統合100 testsが成功した。独立84 PDFs / 585改変拒否も成功し、既存84 PDFはbyte一致する。全featureとBook-2無効のworkspace checkはそれぞれ2分21秒・2分11秒で成功した。全実行が終了し、形式・両repoのdiff空白、元全巻package/原ノ味fontのhash不変も確認した。これらはsource接続と既存PDFの回帰結果であり、caption付き表の実配置完了を示すものではない。

### 14.171. 表キャプションの実幅・実高さの計測（2026-09-10）

共通frame preparationでcaptionへ元tableのcontent幅を渡す。captionの段落は自身のstyleとその幅で組み、表cellの幅に縮めない。元caption event範囲の終端でcaptionのleaf範囲を閉じ、その後のrow/cellとは別の領域として収集する。caption内のtableも元の親tableに結び付け、captionを仮想cellへ置換しない。

`ProductionMeasuredTableCaption`は元のleaf/table子のtop/endと自然高を持つ。captionとセルは同じ高さ累積処理を使用し、実際の行高・marker leading/trailing・余白・入れ子tableの実高さを反映する。入れ子tableの並列セルを直列に足さず、元のrowspan/row計測を先に終えたtableの高さを一度だけ消費する。親tableのrow topはcaptionの高さから始まり、全高はcaptionとrow領域を含む。captionの保持レコードと子extentを共有予算・fingerprintへ含める。

本節では拒否境界をframe preparationから表search kernelの`PendingRegion("table_caption_pagination")`へ移した（後続§14.172で配置・PDF接続を記録する）。計測の成功だけではページの候補・継続位置・脚注予約・配置・PDFを認可しない。既存の入れ子table改ページや脚注内tableの拒否も維持する。次の接続ではcaptionのsource leaf範囲と行群を同じ候補に含め、改ページ後にcaptionをheaderとして再描画しないことを検証する。結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-caption-measurements-design-14171)へ記録する。

検証はヘッダー有無と2段の入れ子を含む統合100 tests（26.96秒）、既存pagination 94 tests（0.21秒）が成功した。独立84 PDFs / 585改変拒否、従来84 PDFのbyte一致、全featureとBook-2無効のworkspace checkも成功した。元全巻package/原ノ味fontのhashは不変で、全実行は終了している。これは実計測と既存対応範囲の回帰検証であり、caption付きtableのページ配置・PDF受入は引き続き必要である。

### 14.172. 表キャプションの改ページ・脚注・実PDF接続（2026-09-10）

計測済みcaptionのleaf範囲を表の候補選択・継続カーソル・共通配置へ接続した。「表の最初のfragment」と「headerを既に描いた状態」を分け、captionだけが続くページではheaderを消費しない。captionの残りと最初のheader/bodyが同居するときは実測位置で配置し、その後のページではheaderだけを繰り返す。captionにはcell roleを付けず、本文照合と脚注需要は元caption/header/bodyのleafを一度ずつ消費する。captionのkeepと分割可能境界、headerの不可分性・本文との接続を共通候補に含める。

captionの選択範囲・headerの状態・配置位置を候補fingerprintへ含め、既存の共有処理量・保持レコード予算で制限する。脚注が収まらない場合の候補縮小では、まだ描いていないheaderの高さを二重に予約しない。従来のcaptionなしtableの符号化は維持した。

新規検証はcaption長短・header有無の4通りと、captionから始まる脚注を後のheaderでも参照するケースを実driver PDFまで通す。4ページの直接候補・元leafの一回消費・captionのcell属性不在・処理量とrecord予算のexact/1不足も検証する。独立検証はTable/Caption/THead/TBodyの順序・構造・MCID・脚注リンクに加え、captionの偽TH化、cell属性付与、親構造の改変を拒否する。別の抽出検査はcaptionにResult、cellにProofという異なる元文字列を用い、8 PDF / 24ページの各ページの出現数を照合した。

統合103 tests、pagination 94 tests、独立94 PDFs（実driver 38件）/329ページ/677改変拒否が成功し、従来84 PDFはbyte一致した。コマンド・ログとビルド出力先の分離は[進捗記録](28-vmb-book-production-progress.md#book-2-table-caption-pagination-and-pdf-design-14172)を参照する。小入力は既存TrueType fixtureによる配置の証拠である。caption内の入れ子table改ページは`nested_table_breaks`、captionの強制改ページは`table_caption_forced_break`として元owner付きで拒否し、脚注内tableの既存制約も維持する。本節時点ではVMBの表Title/Caption/Number producerも未完であった（後続§14.173で接続を記録する）。元原ノ味全巻、公開経路・人手受入は引き続き未完である。

### 14.173. VMBの表Title・Caption・番号と元原ノ味PDF（2026-09-10）

VMBの非公開Book-2 exporterは、表のTitleとCaptionを別々のrich paragraphとして`table.caption`へ出力する。Titleがある場合にもCaptionを捨てず、元のlanguage・装飾・inline math・参照・source位置を共通経路で保持する。captionを出力しない表にはfieldを追加せず、従来の表document/source bytesを維持する。旧exporterは新しい著者fieldと設定を引き続き拒否する。

`BodySettings.TableCaptions`の`ResolvedTableCaption`は明示されたPrefixと元LogicalNumberを対応付ける。番号表示は推測せず、元Numberとの不一致・必要な表示設定の欠落・空prefixを診断する。anchorとprefixは先頭段落に一度だけ置き、`NumberRanges`を実prefix TextのUTF-8範囲へ結び付ける。番号だけの表では生成段落として由来を記録し、著者Title/Captionを捏造しない。著者fieldが空wrapperやbreakだけの場合は、番号があっても拒否する。captionと番号の所有関係・元source範囲はsidecarの再検証と共通予算にも含まれる。

実VMB engineによる3つのinline math、2つのrich caption段落、2列のheader/body、表番号への前方リンクを含む保存済み入力を作成し、元原ノ味fontで一ページPDFまで検証した。Popplerの抽出は元sourceと番号sidecarから得る本文に一致し、9件のcaption/番号改変を拒否した。列間の空きによるPopplerのセル区切りはこのfixtureで明示的に扱い、他の空白を一律削除しない。独立構造検証と描画確認も成功した。

VMBのrendertypaxis全テスト、保存済み9入力を含む統合104 tests、独立96 PDFs（実driver 39件）/331ページ/697改変拒否が成功した。従来94 PDFはbyte一致する。保存済みpackageとPDFのhash、実行コマンドは[進捗記録](28-vmb-book-production-progress.md#vmb-table-title-caption-and-number-export-design-14173)を参照する。これは小規模VMB表の接続であり、表の右/中央揃え、入れ子・強制改ページ、脚注内table、元全巻・公開経路・人手受入は引き続き必要である。

### 14.174. 元セルへの文字スタイル継承とVMB列揃え（2026-09-10）

[ADR-0043](../adr/ADR-0043-book-2-table-cell-inheritance.md)に従い、非公開1.5のtable cellへ任意の`classes`を追加した。省略時は従来のwire bytesを維持し、nullと旧契約でのfield指定はtyped decode/encodeでも拒否する。元cellのnode ID・span・子blockを保持し、classの識別子・順序・予約名を既存の規則で検証する。疑似source nodeは追加しない。

非公開`table_cell`セレクターは`text_align`・`font_family`・`font_size`・`line_height`だけを受理する。元tableから子blockまでの間に継承コンテキストを置き、子段落の明示指定を通常の優先順位で適用する。未使用classや`extends`の祖先にあるgeometry/keep/page指定も拒否する。captionはcellの継承範囲に入れず、domainのcomputed styleとsource flowの両方へ同じ継承を適用する。

VMBのhorizontal-LTR exporterは右/中央揃えを元header/body cellのclassと対応するstyle ruleへ変換する。左揃えは既定状態では追加fieldを省略し、右/中央揃えセル内の入れ子tableでは必要な左揃えリセットを明示する。列順・幅・rich inline・数式・元sourceを維持し、空白の挿入や独自の文字座標配置で揃えない。sidecarへ元cell・column・位置・指定値を保存し、エンコード前のclass削除・置換・無断追加を拒否する。

固定幅と比率幅の異なる3列、caption、繰り返しheaderを持つ2ページfixtureを実PDFへ通した。独立検査はPDFの実glyph matrixと埋め込みCID幅から左/右/中央位置を計算し、captionの不変位置と2ページ目のheaderを照合する。別の元原ノ味VMB fixtureでも4つのheader/body cellと数式を含む行全体の移動量を検証し、Title/Captionの座標と元本文の抽出が不変であることを確認した。

初回検証で入力形式67 tests、syntax 138 tests、pagination 94 tests、保存済み10入力を含む統合106 testsが成功した。独立102 PDFs（実driver 42件）/341ページ/753改変拒否が成功し、従来96 PDFはbyte一致した。VMB exporter全回帰も成功した。ただし継続時に一時ディレクトリの証跡消失を検出したため、再生成した保存先と最新検証結果を[進捗記録](28-vmb-book-production-progress.md#book-2-table-cell-inheritance-and-vmb-alignment-design-14174)で管理する。入れ子tableの分割・表内強制改ページ・脚注内table、全巻・公開経路・人手受入は引き続き必要である。


再生成後も10入力と2つの元原ノ味table PDFは記録済みhashに一致し、統合106 tests・独立102 PDFsの再検証が成功した。現在の証跡は`workspace/target/vmb-design/20260910/table-alignment`に保存する。一時ディレクトリ消失前の96 PDFとのbyte比較は初回観測として記録し、消失後に再比較したとは扱わない。

### 14.175. 表キャプション内の明示的改ページと空白ページ（2026-09-10）

[ADR-0044](../adr/ADR-0044-book-2-table-caption-forced-breaks.md)に従い、非公開Book-2のcaption内`page_break`を元ownerのまま共通ページ選択へ接続した。高さとrow位置だけでは同じ座標にある連続改ページを区別できないため、継続カーソルに次の元caption item位置を保持する。先頭・連続・末尾の改ページを一つずつ消費し、そのfragmentではheader/bodyを開始しない。改ページだけのcaptionと高さゼロのbodyでも、必要な空白ページを維持する。

次の未消費改ページまでを候補の上限とし、強制境界として順位付けする。脚注が収まらない場合は従来どおり手前の候補へ縮小できる。caption内で消費した改ページによって外側の本文itemを進めない。脚注の継続と、表の前後にある通常本文の改ページを共通状態で処理する。最終source照合では非描画の改ページも元itemに対して一回ずつ検査し、描画leaf・cell roleへは変換しない。

source位置・改ページownerを候補fingerprint、継続検証、安定化の再照合へ加え、同じ高さにある古いカーソルの再使用を拒否する。改ページindexは共有record/work/canonical-byte予算へ含める。改ページを含まない表の従来符号化は維持する。直前段落のkeep_with_nextや、figureのkeep_captionにより表全体が保持される場合は、衝突元ownerの`KeepAcrossForcedBreak`で拒否する。新しいtable用style propertyは追加しない。

先頭・連続・末尾・改ページのみ、header有無、脚注継続、keep衝突、元leafの一回消費、処理量とrecordのexact/1不足を検証した。原ノ味を変更せず使う日本語fixtureは7ページで、本文抽出の出現数はページ順に`[0,1,0,1,2,2,2]`となる。最後の3ページは実フォントの実測高に従いheaderとbody 1行ずつを置く。原ノ味の描画も確認した。従来の最小TrueType fixtureはglyphに可視の輪郭がないため、配置・構造・抽出の証拠とし、表示品質の証拠には使わない。

保存済み10 VMB入力を含む統合111 tests、pagination 94 tests、独立112 PDFs（実driver 47件）/395ページ/843改変拒否が成功した。改ページ専用の別抽出は8 PDFs /42ページ/24改変拒否、既存102 PDFと保存済み原ノ味table 2 PDFのbyte一致も成功した。コマンド・失敗した試験の修正経緯・保存先は[進捗記録](28-vmb-book-production-progress.md#book-2-table-caption-forced-breaks-design-14175)を参照する。

§14.172・174で未対応としていたcaptionの強制改ページは本節で接続した。header/body cell内の強制改ページは`table_forced_break`、入れ子table分割は`nested_table_breaks`として引き続き元owner付きで拒否する。脚注内table、元全巻、公開経路と人手受入は未完である。

### 14.176. 並列セルの元contentカーソルと本文セル内の改ページ（2026-09-10）

[ADR-0045](../adr/ADR-0045-book-2-parallel-cell-breaks.md)に従い、非公開Book-2の本文セル内page_breakを、rowspanを含まない表で共通ページ選択へ接続した。元セルごとに次のcontent位置を保持し、行の中で最初に到達する改ページまでを各セルの容量とする。隣のセルの次の行が収まらない場合は、その行を未消費のまま次ページへ残す。同じ境界に到達した別セルの改ページは同じページで消費し、同一セル内の連続改ページはそれぞれ別ページを進める。表の最後が改ページの場合も後続の空白ページを維持する。

継続位置は計測元に結び付いた表search内へ保持する。カーソルは保持位置の番号と、全セルの元content位置から得たfingerprintを持つ。候補の再検証と安定化では内容のfingerprintを照合し、保持順序の番号をcanonical bytesへ混ぜない。保持配列の複製、各セルの走査、hash用領域と候補を共有record/work/canonical-byte予算へ含める。高さが同じでも消費済み改ページが違う古いカーソルを拒否する。この経路のoffsetは累積fragment高であり、各セルの元位置を代用する座標ではない。

一行の全セルが終了したら、残ったページ領域へ次の元rowを配置できる。colspanは元の幅と所有関係を維持する。headerは本文を開始できるfragmentでだけ容量を使い、継続するrowページで反復する。captionは別の元領域として保持し、最初のrowと同居できる。caption末尾のkeep_with_nextは本文の配置成功まで仮選択とし、収まらない場合はcaptionの手前の合法な分割へ戻る。同時改ページの全ownerは本文source照合で一度ずつ消費し、代表ownerだけを残して他を捨てない。非描画の改ページをcell描画leafへは変換しない。

異なる行高の左右セル、同時・連続改ページ、空セル、通常／強制改ページを含むcaption、caption keepの同居と手前への戻し、繰り返しheader、脚注継続、次のrowとcolspanを実driver PDFで検証した。元原ノ味の3ページ小入力はページ順に「左側」「左側・右側」「右側」と抽出され、2ページ目の可視描画も確認した。処理量・record予算のexact/1不足、同じ高さの古いカーソル、keep衝突と未対応sourceのownerも検証した。

最終回帰は保存済み10 VMB入力を含む統合117 tests、pagination 94 tests、独立136 PDFs（実driver 59件）/479ページ/1,011改変拒否が成功した。別のセル本文抽出は22 PDFs /76ページ/88改変拒否、既存112 PDFと保存済み原ノ味table 2 PDFのbyte一致も成功した。原ノ味PDFのhash、コマンドと保存先は[進捗記録](28-vmb-book-production-progress.md#book-2-parallel-cell-breaks-design-14176)を参照する。

旧profileと改ページを含まない表は従来経路・符号化を維持する。rowspanを含む表のセル改ページとheader内改ページは、元break ownerの`table_forced_break`として引き続き拒否する。rowspan対応、header内改ページ、入れ子table分割、脚注内table、元全巻、公開経路・人手受入は設計全体の残件であり、本節の小入力をそれらの完了証拠とは扱わない。

### 14.177. rowspanの行高継続と後続行の改ページによる再計算（2026-09-10）

[ADR-0046](../adr/ADR-0046-book-2-spanning-cell-breaks.md)に従い、本文セル内の明示的改ページをrowspanを含む表へ拡張した。元の計測済み行高と、各セルの未消費content位置を別々に保持する。結合セルの行が隣の行境界を越える場合も、次の元rowは自身の列でその境界から開始できる。同一候補では各セルの配置済み終端を保持し、同じセルの次の内容をその終端より手前へ置かない。物理ページをまたぐ未消費内容はセルごとのカーソルで再開する。

現在rowの残りの計測高を保持状態とfingerprintへ含める。空のrowや余白の進行を元sourceの消費と区別し、空の行境界を進めただけではcaptionのkeep_with_nextを満たさない。captionと本文が同居できない場合は、captionの合法な手前の境界へ戻す。本文を開始できなかった候補にheaderだけを残さない。

後のrowで、既に仮配置した結合セルの行より早い改ページが見つかった場合は、その候補を破棄して元カーソルから小さい容量で再計算する。毎回容量を厳密に減らし、予算を再設定・返却しない。候補縮小では空白の行高を1固定小数点単位ずつ列挙せず、セル・caption・headerの実終端へ移る。再試行、セル走査、配置終端／停止状態の一時配列、残り行高の保持を共有record/work/canonical-byte予算へ含める。元rowspan・列・cellの意味所有関係は変更しない。

異なる行高の結合セル、後続rowの改ページで先行候補を戻す入力、3行結合と高さゼロの中間row、同時改ページ、空白ページだけの結合セル、header反復、captionとkeepの巻き戻し、通常セル／結合セルからの脚注を実driver PDFで検証した。独立したPDF文字座標検査は、左右の列と、結合セルの行高の途中で隣の次行が始まる位置を照合する。結合セルの参照・脚注・注釈が未配置の1ページ目へ先出しされず、2ページ目から現れることも確認した。

最終回帰は保存済み10 VMB入力を含む統合120 tests、pagination 94 tests、独立158 PDFs（実driver 70件）/561ページ/1,157改変拒否が成功した。別のrowspan検証は18 PDFs /56ページ/6件の実文字座標検査/86改変拒否、既存136 PDFと保存済み原ノ味table 2 PDFのbyte一致も成功した。元原ノ味の3ページ入力では「左側」「左側・右側・左側」「右側」の抽出と可視描画を確認した。コマンド・hash・修正経緯は[進捗記録](28-vmb-book-production-progress.md#book-2-spanning-cell-breaks-design-14177)を参照する。

§14.176時点のrowspanを含む本文セル改ページの拒否を本節で解除した。旧profileとセル改ページを含まない表の経路は維持する。header内の強制改ページ、入れ子table分割、脚注内table、元全巻、公開経路・人手受入は引き続き必要であり、これらの完了とは扱わない。

### 14.178. 初回ヘッダーの元改ページと後続の反復描画（2026-09-10）

[ADR-0047](../adr/ADR-0047-book-2-header-source-breaks.md)に従い、非公開Book-2のheader cell内page_breakを共通ページ選択へ接続した。最初のheader走査では、元セルのcontent位置と結合行の残り高さを本文と同じ経路で保持する。最初に到達する改ページまでを元THセルの部分範囲として配置し、先頭・連続・末尾の改ページ、改ページだけの空白ページを維持する。同じ境界に到達した複数セルの改ページも元ownerごとに一度だけ消費する。

元headerの消費が終わった後の反復headerは、全体を自然な高さで描画するpagination artifactとする。元の改ページ命令を再実行せず、THや元leafを複製しない。脚注参照も再び要求しない。部分headerの元source範囲と、反復headerの描画範囲を別に扱い、非描画のpage_breakを文字leafへ変換しない。header末尾の改ページの次から本文を始める場合は、そのページに全体の反復headerを置く。

元header内の明示的改ページがない境界では、headerの途中や本文を開始できない最後の部分だけを自然改ページで残さず、次ページへ送る。反復headerの全高が版面を超える入力は、従来のTableHeaderOversizeで拒否する。本文が空、または初回headerの残りと本文が同じページで終了する場合は、header高を二度加算しない。元header完了状態・セルカーソル・HEADBRK1識別子を候補のcanonical bytesへ結び付け、保持配列の複製・結合・再試行を共有予算へ含める。

header/bodyそれぞれのrowspan、両領域の改ページ、caption改ページ、脚注継続、空body、keep衝突と反復headerの大きさ制限を検証した。直接再生は元sourceの一回消費と処理量・record予算のexact/1不足を確認する。別の実PDF検査は、各ページの文字列、左右のglyph matrix、反復headerのartifact属性を照合し、表示文字の欠落・重複・移動やartifact/sourceの入れ替えを拒否する。

保存済み10 VMB入力を含む統合124 tests、pagination 94 tests、独立184 PDFs（実driver 83件）/679ページ/1,319改変拒否が成功した。header専用検査は24 PDFs /102ページ/22件のglyph・artifact検査/156改変拒否で、従来158 PDFと原ノ味table 2 PDFはbyte一致する。元原ノ味の5ページ出力は、初回headerが最初の2ページで完了し、3ページ目以降で全体を反復する。抽出と可視描画を確認した。保存先・hash・失敗修正の記録は[進捗記録](28-vmb-book-production-progress.md#book-2-original-header-breaks-design-14178)を参照する。

§14.177まで残っていたheader内改ページの拒否を本節で解除した。入れ子table分割、脚注内table、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・人手受入は引き続き必要である。本節の小規模fixtureを設計全体の完了証拠とは扱わない。

### 14.179. 入れ子表に向けた外側の表カーソル（2026-09-10）

入れ子表の分割に先立ち、Book-2の外側のページカーソルを元table階層へ結び付けた。従来は表終了時にtable indexを一つ増やしていたため、親の直後に保持される子tableを次の外側の表として再処理してしまう構成だった。元sourceのpreorderから各subtreeの終了indexと直前のroot tableを保持し、外側の表を消費した後は全子孫を飛ばす。候補評価・自動ページ列挙・keep判定で同じ関係を用いる。child tableは外側の通常本文範囲へ出さない。

階層は高さやleaf数から推定しない。空の表や空の入れ子でも元parent indexに従い、4,096段の試験を再帰呼び出しなしで処理する。前方・自己parent、終了済みparentの再開、body/definitionの異なる親子、範囲外・重複sourceを元owner付きで拒否する。保持配列と走査は共有record/work予算を使い、exact/1不足を検証する。入れ子がない場合は追加の階層配列を確保せず、従来のtable index順序を維持する。

pagination 98 tests、保存済み10 VMB入力を含む統合124 testsが成功した。従来184 PDFと原ノ味table 2 PDFはbyte一致した。この変更は階層カーソルの基盤であり、子表のfragmentを親へ返す分割・配置はまだ接続していない。`nested_table_breaks`診断は維持し、入れ子表のPDF対応や設計全体の完了とは扱わない。コマンドと証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-hierarchy-cursor-design-14179)を参照する。

### 14.180. 本文セル内の子表fragmentと親セルの継続（2026-09-10）

[ADR-0048](../adr/ADR-0048-book-2-nested-body-fragments.md)に従い、本文セル内の入れ子tableを共通ページ候補とPDFへ接続した。親側にcaption・header・rowspan・未接続keep連鎖がない行を対象とし、各親セルは次の元content位置と未完の子tableカーソルを保持する。子表は自身の既存探索を使い、実選択高、caption、元セルowner、反復header属性と全改ページownerを親へ返す。子表の全高を一個の不可分領域として扱わず、左右の親セルの高さも加算しない。

子表の先頭・末尾spacingは最初・最後のfragmentに一度だけ適用する。子表全体が候補に収まっても末尾spacingが収まらない場合は、同じ未消費カーソルから小さい容量で再評価する。子表の終了後、残り領域へ次の子表や親の次行を置ける。別の親セルにより先行する子表候補より早い改ページが見つかった場合は、仮選択を破棄し、元親状態から厳密に小さい容量で再試行する。同時改ページの全ownerを一度ずつ消費する。

元sourceと配置leafを別に集め、子表の反復headerを元THとして再消費しない。子表captionを親の疑似cellへ変更せず、元のTable/Caption/TR/TH/TD階層と解決済み列幅を保持する。親子の探索は唯一のrecord/work予算を受け渡し、失敗時も返却・再設定しない。保持する子カーソルの意味状態をNESTPOS1、実際の元source・配置leafとその件数をNESTFRG1へ結び付ける。arena上の保持番号はcanonical bytesへ含めない。候補縮小は空白高を一固定小数点単位ずつ走査せず、実leaf終端へ進む。

子表の自然・強制改ページ、通常／内部改ページ付きheader、caption、rowspan、同じ親セル内の複数子表、左右の並列子表、3段の親wrapper、親の後続行、次の外側table、spacing、脚注を実driver PDFで検証した。元sourceの一回消費とrecord/work予算のexact/1不足、親による早い改ページの巻き戻しも確認した。definition内tableの読み取り専用leaf照会では、本文専用配列を参照していた問題を修正し、共有body/definition streamへ照合する。これは脚注tableのページ配置の接続を意味しない。

保存済み10 VMB入力を含む統合128 tests、pagination 98 tests、独立216 PDFs（実driver 99件）/787ページ/1,519改変拒否が成功した。入れ子専用検査は28 PDFs /94ページ/224改変拒否で、各物理ページの文字、元段落のglyph位置、子表のartifact列を照合する。従来184 PDFと原ノ味table 2 PDFはbyte一致した。元原ノ味の3ページ小入力は「左側・左側右側」「左側・右側」「右側」と抽出され、子表の2列が親の左セル内へ置かれる可視描画を確認した。コマンド・hash・保存先は[進捗記録](28-vmb-book-production-progress.md#book-2-nested-body-fragments-design-14180)を参照する。

親caption/header内の入れ子、親rowspan、親セルのkeep連鎖、脚注内tableは引き続き元owner付きで拒否する。これらの接続、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・人手受入は設計全体の残件である。本節の部分的な入れ子対応を、入れ子表全体や設計全体の完了とは扱わない。

### 14.181. 親セルのkeep連鎖と子表終端の再選択（2026-09-10）

[ADR-0049](../adr/ADR-0049-book-2-nested-parent-keeps.md)に従い、入れ子tableを含む親セルのkeep_with_nextを接続した。各セルで最後の合法な選択位置を保持し、後続内容が入らない場合は、そのセルのsource・描画leaf・子カーソル・改ページowner・進行状態を戻す。他の親セルの選択は保持する。同じセルの末尾にあるkeepを隣のセルへ連結しない。

直前段落のkeepは子表fragmentの開始によって満たす。子表自身のkeepは最終fragmentと次の元contentに適用し、途中のfragmentを禁止しない。子表の末尾だけが収まり後続段落が入らない場合は、元セル開始状態へ戻し、子表の実leaf終端に基づく小さい容量で再選択する。子表の手前で分割することで最終fragmentと後続段落を次ページへ同居させる。子表終端からさらにkeep付き段落へ続く連鎖も、合法な終端まで仮選択として保持する。破棄した探索のrecord/work予算は返却せず、子カーソル保持領域も巻き戻さない。

keep付き段落／子表の直後にある明示的改ページは、直前の元ownerでKeepAcrossForcedBreakとする。figureのkeep_captionで保持される親tableは全体が収まる候補だけを選び、子孫の元改ページも親table ownerで衝突診断する。ページ残量に収まらない場合は次ページへ送り、全版面でも入らない不可分連鎖はOversizeとする。未配置の脚注参照を含む仮選択を戻した場合、脚注と注釈も先のページへ残さない。

保存済み10 VMB入力を含む統合132 tests、独立238 PDFs（実driver 110件）/843ページ/1,653改変拒否が成功した。keep専用検査は22 PDFs /56ページ/176改変拒否で、各物理ページの文字、元列位置、脚注参照と注釈の開始ページを確認する。元sourceの一回消費、破棄候補を含むrecord/work予算のexact/1不足も検証した。従来216 PDFと原ノ味table 2 PDFはbyte一致する。元原ノ味の実測行高を使う2ページ入力では、子表の最終行と後続段落の同居・抽出・可視描画を確認した。コマンド・hash・保存先は[進捗記録](28-vmb-book-production-progress.md#book-2-nested-parent-keeps-design-14181)を参照する。

§14.180で未接続だった親セルのkeep連鎖を本節で接続した。親caption/header内の入れ子、親rowspan、脚注内table、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・人手受入は引き続き必要であり、設計全体の完了とは扱わない。

### 14.182. 親キャプション内の子表と本文開始の分離（2026-09-10）

[ADR-0050](../adr/ADR-0050-book-2-nested-caption-fragments.md)に従い、header・rowspanを持たない親tableのcaptionへ入れ子tableを接続した。キャプション用に次の元contentと未完の子tableカーソルを保持し、本文セルの並列カーソルとは別に進める。本文内の子表を持つ親に通常のcaptionがある場合も、同じ経路で扱う。追加する保持位置は内部の継続状態であり、元captionにTDや疑似source ownerを付与しない。

captionを先に選択し、その子表の途中や元改ページで終わった場合は親本文を開始しない。captionが終了したページの残り領域には元body rowを置ける。先頭・連続・末尾の改ページと、本文セルが空の場合の末尾改ページ後の空白ページを維持する。子表は元caption全幅を使い、自身のcaption、header反復、実セルowner、列幅、前後spacingと改ページを保持する。複数のcaption子表と、caption内にさらにcaptionを持つ親子も同じ探索を使う。

最後のcaption contentのkeep_with_nextは元本文の開始まで仮選択とし、本文が入らない場合は元captionカーソルから手前の位置へ戻す。子表の最終fragmentを次ページへ送り、親本文と同居させる場合もある。空行の幾何だけではkeepを満たさず、本文全体が空なら後続保持を要求しない。子表の直後の元改ページとの衝突は子表ownerで拒否する。未配置captionの脚注参照・脚注・注釈を先のページへ出さない。追加カーソル、候補の破棄・再試行も共有予算に含める。

保存済み10 VMB入力を含む統合136 tests、独立272 PDFs（実driver 127件）/955ページ/1,961改変拒否が成功した。caption専用の検査は34 PDFs /112ページ/272改変拒否で、物理ページの文字、元列位置、子headerのartifact属性、脚注参照・注釈の開始ページを照合する。元sourceの一回消費とrecord/work予算のexact/1不足も確認した。従来238 PDFと原ノ味table 2 PDFはbyte一致する。元原ノ味の4ページ出力では、captionの子表が3ページ目で終わり、元親本文が4ページ目から始まることを抽出・可視描画で確認した。コマンド・hash・修正経緯は[進捗記録](28-vmb-book-production-progress.md#book-2-nested-caption-fragments-design-14182)を参照する。

§14.180・181で残っていた親captionの入れ子を本節で接続した。親header、親rowspan、脚注内table、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・人手受入は引き続き必要であり、本節を入れ子table全体や設計全体の完了とは扱わない。

### 14.183. 親ヘッダーの元sourceとセルを持たない反復描画（2026-09-10）

[ADR-0051](../adr/ADR-0051-book-2-nested-header-regions.md)に従い、入れ子tableを持つ親headerを共通ページ選択へ接続した。初回headerは元captionの後、元row・content・子tableカーソルから進める。自然な途中位置でheaderを残したり、本文を開始できない完成headerだけを残したりせず、仮選択したheader全体を元状態へ戻す。先行captionの合法なprefixは保持し、caption末尾のkeepがある場合はさらに手前の分割へ戻す。元headerまたは子tableの明示的改ページでは、本文を始めずに元headerの部分範囲を終了できる。

元header完了後は、元の自然な全高でheaderを反復する。子tableのCaption・cell・列幅をそのまま走査し、元改ページを実行せず、描画leafだけを追加する。子表の脚注要求や意味sourceを再消費しない。反復headerの全高には既存のTableHeaderOversize制限を適用する。

header内の子table captionにはセルがないため、反復属性をセルownerから推定する経路を改めた。セルを持たない反復leafの位置を別に保持し、全fragmentを元cell役割と反復flagの組で照会する。source照合、数式・数式番号、文字、画像、マーカー、アンカーと非描画行を同じ属性へ結び付ける。PDFでは元CaptionをTH/TDへ変更せず、反復する描画をPagination/Header artifactとする。反復アンカーを新しいdestinationへせず、脚注注釈も複製しない。追加保持位置、headerの巻き戻し、自然な反復走査も共有予算へ含め、破棄した試行の予算は返却しない。

保存済み10 VMB入力を含む統合139 tests、pagination 99 tests、独立300 PDFs（実driver 141件）/1,021ページ/2,191改変拒否が成功した。専用検査は28 PDFs /66ページ/250改変拒否で、元headerと反復caption/headerのglyph、列位置、本文の開始高と脚注注釈の一回生成を照合する。header内の画像・アンカー・空段落の反復、元sourceの一回消費、record/work予算のexact/1不足も確認した。従来272 PDFと原ノ味table 2 PDFはbyte一致する。

元原ノ味の2ページ小入力では、親headerの左側に子tableのcaptionと2列、その右側に元header cellを反復し、下へ元body rowを配置することを抽出と可視描画で確認した。コマンド・hash・テスト補助の更新経緯は[進捗記録](28-vmb-book-production-progress.md#book-2-nested-header-regions-design-14183)を参照する。親rowspan、脚注内table、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・人手受入は引き続き必要であり、設計全体の完了とは扱わない。

### 14.184. 入れ子表を含む親rowspanの行高と子カーソル（2026-09-10）

[ADR-0052](../adr/ADR-0052-book-2-nested-spanning-rows.md)に従い、親のrowspanを入れ子table探索へ接続した。元行の残り高を、各セルの次content・未完子tableカーソルから分離して保持する。現在の元行を覆うセルを並列に選び、その行で終わるセルが完了し、元の最小行高が収まった場合に次の行へ進む。後の行まで結合するセルの子表は現在行の下へ配置できるが、後続行でより早い元改ページが見つかった場合は親の元カーソルから小さい容量で再選択する。先に描画した子表や破棄した試行の予算を確定候補へ混入させない。

元headerの行結合にも同じ継続を使う。先頭・末尾・連続する元改ページを一度だけ消費し、元headerが完了した後は自然な全高で反復する。header全体を戻す際は元行の残り高も復元する。captionの継続は実セルと分離したまま保持し、子captionを含む反復headerの描画属性・元owner・列位置を維持する。残り行高をNESTROW1として継続fingerprintへ含め、追加配列・行走査・再選択は共有record/work予算へ課金する。rowspanがない既存経路の継続符号化は変更しない。

保存済み10 VMB入力を含む統合142 tests、pagination 99 tests、旧表49 testsが成功した。独立検査は332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否、専用検査は32 PDFs /88ページ/224改変拒否である。左右それぞれの結合セル、後続行・同時改ページ、空の中間行、caption keep、元／反復header、反復子caption、空本文、脚注の座標とsourceを検査した。caption内だけに子表がある親rowspanについても元sourceの一回消費とrecord/work予算のexact/1不足を確認した。従来300 PDFと原ノ味VMB table 2 PDFはbyte一致する。

元原ノ味では、後続行の改ページにより1ページ目に親の右セルを置き、子表全体と次の右セルを2ページ目へ送る2ページPDFを確認した。和文baselineは元フォントのhhea（1151/-286、UPEM 1000）と12ptサイズから独立計算して照合し、実アウトラインの可視描画も確認した。証跡・hash・検証用版面の調整経緯は[進捗記録](28-vmb-book-production-progress.md#book-2-nested-spanning-rows-design-14184)を参照する。脚注内table、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・人手受入は未完であり、本節で設計全体を完了とはしない。

### 14.185. 脚注定義内の再帰的な表断片と元ストリーム番号（2026-09-10）

脚注内tableの共通ページ配置に向け、元定義に属する入れ子tableを表の再帰探索へ接続した。本文専用配列への参照を本文・脚注を含む共有の元item列へ変更し、子表のcaption/header/rowspan/keep/強制改ページを同じカーソルと共有予算で選択する。脚注tableの最大高は元footnote frameから取得する。これは表断片の選択であり、separatorを含む脚注領域の確定ではない。

BookV2TableSourceLeafは元definition番号（本文はNone）、実cell owner（captionはNone）、定義内ローカルitem番号、相対top、独立した反復flagを保持する。source_leaf_rangesは消費した元source範囲を同じストリームのローカル番号へ変換し、source_placement_leavesはcaptionとcellの描画を同じ所有形式で照会する。元範囲外の番号はReceiptMismatchとし、反復headerを意味sourceへ追加しない。共有配列のglobal番号をそのまま脚注のローカル番号として扱うことを防ぐ。既存の本文配置もこのAPIを使う。

本文2段落と先行脚注3段落の後へ表を持つ第2脚注を置き、自然／強制改ページ、header、caption、深い入れ子、親rowspan、captionを含む親header-rowspanの7形式を検証した。元定義の全sourceを一回消費し、描画leafが同じ定義内の元itemを指すこと、相対高が選択範囲へ収まること、record/work予算のexact/1不足で決定的に成功／失敗することを確認した。統合143 tests、pagination 99 testsと独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。

脚注の従来の連続itemカーソルを、この表断片で置き換える領域選択・配置は未接続である。非連続な元セルの参照要求、複数脚注の最小予約と依存先の巻き戻し、反復headerの描画と脚注番号の一回生成、source closureと安定性照合を接続する必要がある。既存のtable_footnote_definition診断は共通ページ探索に残し、並列セルを平坦な脚注本文として出力しない。本節の検証を脚注tableのページ／PDF成功とは扱わない。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-definition-table-fragments-design-14185)を参照する。

### 14.186. 脚注定義の表カーソルと選択済みsourceからの要求（2026-09-10）

[ADR-0053](../adr/ADR-0053-book-2-definition-table-demands.md)に従い、定義tableの実カーソルと脚注要求のsnapshotを一つの探索ownerへ結び付けた。元本文の参照項目から対象脚注を要求し、tableの選択と依存脚注の要求で同じrecord/work残量を受け渡す。版面・separator・脚注番号の描画を確定する前のsource状態であり、共通ページ探索のtable_footnote_definition診断は維持する。

非連続な並列セルを、脚注の連続item範囲として要求しない。表の元意味leaf範囲を定義内ローカル番号へ照合し、参照glyphを運ぶ元itemが選択に含まれる場合だけ、その参照先を要求する。複数itemにまたがる参照の一部分だけを含む候補は採用しない。反復headerは意味leafを増やさず、参照要求の由来にならない。次の要求状態をforkして更新するため、候補を捨てても元状態に依存要求は残らない。

tableの終端と脚注定義の終端を区別する。後続の通常itemや別のroot tableがある場合は定義をPendingに保ち、leafの終端番号を共有する空の後続tableも省略しない。自己参照を含む最後のfragmentでは、現在の定義を完了してから参照要求を処理し、完了した脚注を再queueしない。未消費の前置内容を飛ばす候補、別の探索器や別のsnapshotの流用はReceiptMismatchで拒否する。

早期／後半セルの参照、並列セル、反復header、終端の自己参照、未消費prefix、通常suffix、空の後続tableを検証した。元sourceの一回消費、候補破棄後の要求状態、定義完了の区別、元本文参照の欠如、所有者・branch不一致、0容量試行の非返却予算、record/workのexact/1不足を確認した。統合144 tests、pagination 99 tests、独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。

この部品は対象tableの実際の定義境界から開始する。前後の通常内容を含む脚注全体の候補列、複数定義の予約と依存先の巻き戻し、ページ配置・反復header・脚注番号・source closureへの接続は引き続き必要であり、脚注tableのPDF成功を主張しない。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-definition-table-demands-design-14186)を参照する。

### 14.187. 脚注定義の通常本文・表・改ページを含む候補列（2026-09-10）

本文と表の順序・間隔・keepを扱う共通処理を、ページの脚注予約から分離した。従来の本文探索は同じsource選択の後に既存の予約処理を実行する。脚注定義では、元定義のローカルitem番号、文書全体の元table番号、未完tableカーソルと要求snapshotを保持し、通常本文と複数のroot tableを同じ候補列として選ぶ。表を持たない定義にも同じ通常本文処理を使用できる。

先頭では外側の段落間隔を入れず、同じ候補内では直前のspace_afterと次のspace_beforeを加える。段落自身のleading・trailingを消費高に含め、表は元並列セルの選択高を使う。前置本文のkeepや表末尾のkeepを破る候補は採用しない。空のroot tableは元table番号で進め、同じleaf終端番号に続く別の空表を飛ばさない。通常本文の先頭・末尾・連続改ページは元itemを一度だけ消費し、その候補を終了する。表内部の改ページの後へ同じ候補内の通常本文を追加することも拒否する。

定義のkeep事前検査は元root tableを直列の一要素として扱う。平坦化された別セル同士の境界にkeep違反を作らず、caption・セル内の検査は表探索に任せる。通常本文またはroot tableのkeepと直後の元改ページの矛盾はKeepAcrossForcedBreakとする。選択した通常本文・表の元参照だけから要求を作り、未消費suffixや空表があれば定義をPendingに保つ。候補の破棄で入力snapshotを変えず、失敗した試行のrecord/work予算も返却しない。

12形式で通常本文・表の高と間隔、複数・空表、表なし定義、自然な表継続、元改ページ、自己参照、keepの採用／拒否、別セルの境界、元sourceの一回消費、探索owner・branch不一致、record/work予算のexact/1不足を検証した。統合145 tests、pagination 99 tests、独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。

この候補列はsourceと要求の選択であり、候補の自動列挙・順位付け、複数脚注の同時予約・依存先の巻き戻し、実ページへの配置・反復header・脚注番号・source closureは未接続である。共通ページ探索のtable_footnote_definition診断を維持する。脚注tableのPDF成功や、元全巻・公開経路・管理ホスト・著者および人手受入の完了は主張しない。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-mixed-definition-sources-design-14187)を参照する。

### 14.188. 脚注定義候補の自動列挙と空表を含む同点処理（2026-09-10）

[ADR-0054](../adr/ADR-0054-book-2-definition-candidates.md)に従い、通常itemの境界と、最後に含める表の合法な分割位置を、指定された脚注容量の中で全列挙する。先行する表が完了し、元改ページで終了していない場合は、その後の通常本文や別のroot tableまで候補を伸ばす。表の分割位置を戻す処理は本文と共有し、脚注を含む元global item列から入れ子leafとrowspanの境界を読む。

評価値は共通の段落・見出し・余白コストから求める。同点の定義候補は、元item、root table番号、未完tableの位置の順に、より先へ進むものを優先する。0容量でも連続する空表を元番号で消費でき、空の後続tableが高さのちょうど境界に残る場合も同点だけで次の領域へ送らない。0容量のコスト計算では除算を行わない。本文側の既存の同点規則は維持する。

候補集合は評価順の全採用可能branchと、調べた境界数・容量・元snapshotを保持する。元参照の一部だけを含む候補は採用せず、各採用候補の要求状態を別に保持する。呼出側は最良候補を取り出すか、同じ入力snapshotに結び付いた候補を後から比較できる。最初の候補が収まっていても列挙を省略せず、max_page_break_lookback、record、workのいずれかを使い切ればエラーとする。候補の破棄や再選択で元状態を変えず、予算も返却しない。

22形式の検証では、20形式の採用可能な元定義に通常本文・表・caption・header・親rowspan・深い入れ子・元改ページ・空表・自己参照を含め、自動継続とsourceの一回消費を確認した。別の2形式はkeep矛盾と、早期の採用可能候補がある場合のlookback不足を拒否する。早いセル・後半セル・反復headerの参照についても、各候補が選択した元sourceと要求状態を照合した。統合145 tests、pagination 99 tests、独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。

候補の自動列挙・順位付けは接続したが、これらの継続を脚注要求queueへ保持する処理、複数脚注の同時予約・依存先の巻き戻し、実ページ配置・反復header・脚注番号・source closureは引き続き必要である。共通ページ探索のtable_footnote_definition診断を維持し、本節を脚注tableのPDF成功、元全巻・公開経路・管理ホスト・著者および人手受入の完了とは扱わない。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-automatic-definition-candidates-design-14188)を参照する。


### 14.189. 共通脚注要求キューに保持する表の継続状態（2026-09-10）

[ADR-0055](../adr/ADR-0055-book-2-shared-definition-queue.md)に従い、共通のpending脚注カーソルへ定義内item番号、次の元root table番号、未完tableの全カーソル、元定義マーカーの消費状態を保持した。並列セルが進んでも連続item番号が変わらない場合、空表だけを消費する場合、先頭の元改ページで番号が未消費の場合を区別する。

全定義のtable contextは元preorderを一度走査して準備し、一つの不変hierarchyと共通record/work予算を使う。定義候補の評価器は共通要求ownerと対象contextを借り、別脚注を処理した後も元のtable arenaとカーソルから再開する。失敗時も同じcontextを戻し、消費した予算を返却しない。候補の所有権には元snapshotに加えて定義番号を結び付け、同じsnapshot内の別脚注への流用を拒否する。

22形式の定義検証へ共通queueでの再生を追加した。未完表を残して依存脚注を処理する順序、全カーソルの保持、元sourceの一回消費、反復headerからの再要求防止、自己参照、空表・元改ページと番号状態、record/workのexact/1不足、keep矛盾、lookback失敗後の小さい候補への回復を確認した。統合145 tests、pagination 99 tests、独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。

共通queueへの継続保持を接続した。複数脚注の同時予約・依存先の巻き戻し、物理配置・反復header・番号・source closureは引き続き必要である。旧連続item用領域選択へ混在カーソルを渡す経路と、本文ページ探索のtable_footnote_definition診断を維持する。番号のsource消費は描画receiptではなく、本節を脚注tableのPDF成功や設計全体の完了とは扱わない。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-shared-definition-queue-design-14189)を参照する。


### 14.190. 表を保持する共通脚注断片と順次領域選択（2026-09-10）

[ADR-0056](../adr/ADR-0056-book-2-mixed-footnote-regions.md)に従い、既存BookV2FootnoteFragmentSelectionへ元の通常item／table選択と次の要求snapshotを保持し、同じregion kernelから混在定義を順に選択するようにした。定義が完了すれば残り容量へ次のpending脚注を選び、overflowまたは元改ページで領域を終了する。tableの前後spacingと継続状態を参照し、通常item番号の末尾に空表が残る場合も元root番号から進める。

断片は選択した元参照、初回番号、実継続カーソル、高さと末尾spacingを照会できる。tableの元leafを完全に含む参照だけを保持し、反復headerから元参照を増やさない。参照の保持とコピー走査も共通record/work予算へ課金する。advanceは元snapshotを検証して候補の導出済み要求状態をforkし、脚注要求を再実行しない。

並列tableを含む断片には連続item範囲がないため、staging専用items／consumed_range APIをResultへ変更し、混在断片に対する照会を拒否する。実sourceと描画情報は保持したmixed partsから読む。既存の通常本文用配置・安定性照合はエラーを伝播し、空の配列や全table範囲へ置き換えない。共通のrequired／dependency領域と本文ページ準備には診断を残す。

22形式を順次領域で最後まで再生し、元sourceの一回消費、番号の一回選択、元参照、断片間隔、容量、強制終了、record/workのexact/1不足を確認した。0容量で連続する先頭空表を消費して番号を未消費のまま再開する場合と、古いsnapshotへのadvance拒否も検証した。統合145 tests、pagination 99 tests、独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。

本節で共通の順次領域選択を接続したが、複数脚注の同時最小予約・依存先の巻き戻し、物理配置・反復header・番号描画・source closureは残る。領域の候補を脚注tableのページ／PDF成功とは扱わない。残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・著者および人手受入も引き続き必要である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-mixed-footnote-regions-design-14190)を参照する。


### 14.191. 表を含む複数脚注の予約と依存候補の巻き戻し（2026-09-10）

[ADR-0057](../adr/ADR-0057-book-2-mixed-footnote-reservations.md)に従い、既存のbounded dependency stackへ定義の実mixed候補列を保持し、同じ要求snapshotから別候補を試せるようにした。各frameは残りの候補iteratorを持ち、後続脚注が収まらない場合はそのbranchだけを戻す。他の脚注の全カーソルを保持し、table arenaや累積record/work予算を初期化・返却しない。

開始時にpendingだった各定義の予約と、選択内容から新たに生じた依存先まで収める探索を区別する。select_required_regionは開始時の需要だけを対象とし、新たな要求を次の状態へ残す。共通body fitが未開始の参照先を見つけた場合は、同じstackで依存先を含めて候補を戻す。一つのbranchで各定義を一度だけ訪れ、循環参照を再帰展開しない。元改ページの後へ別の必要脚注を置かず、確定前に保持された全参照の開始状態を検査する。

並列セルの片方だけが進み、元の脚注番号を含む最初のleafが未選択の場合、その断片を初回予約に使わない。明示的な強制改ページだけの非描画進行は維持するが、依存先でその改ページを消費しただけでは参照先開始を満たさない。混在カーソルでは連続item番号が0かどうかでなく、元番号の消費flagで開始状態を照合する。

既存22形式のうち20採用形式をrequired領域でも全sourceが終わるまで再生し、2拒否形式のkeep/lookback診断も維持した。さらに後半参照・早いセル参照・反復子header・循環・本文から2脚注の同時要求・強制改ページだけの依存先という6形式を、実計測から求めた容量で検証した。表全体だけなら入るが依存先を加えると入らない場合に表を短く選び直し、破棄した参照要求を残さないこと、実継続・元sourceと番号の一回消費、separatorを含むfit矩形、record/workのexact/1不足を確認した。

統合146 tests、pagination 99 tests、独立332 PDFs（実driver 157件）/1,109ページ/2,397改変拒否が成功し、従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。共通の複数脚注予約とbody/footnote fitは接続したが、本文table contextの範囲分離、実ページ選択、脚注内table・反復header・番号の物理配置、安定性とsource closureは引き続き必要である。本文ページ準備の診断を維持し、脚注tableのPDF成功とは扱わない。残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・著者および人手受入も未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-mixed-footnote-reservations-design-14191)を参照する。


### 14.192. 脚注内の表の実ページ・配置・source closureとPDF（2026-09-10）

[ADR-0058](../adr/ADR-0058-book-2-definition-table-pages.md)に従い、本文と各脚注のtable contextを一つの元hierarchy・record/work予算から準備する。本文探索と本文完了は元preorderの本文範囲に限定し、脚注tableを本文の残件に数えない。元global table番号と定義内item番号を維持し、本文と脚注に入れ子表が同時に存在する入力を実ページ選択へ接続した。tableを持たない従来経路の予算と出力を維持する。

選択済みmixed partsの通常本文とtable leafを実offsetへ配置する。本文と脚注は元cell・caption・反復属性を持つleaf配置処理を共有し、元改ページは描画せず一度だけsource消費する。0高さの空表も元root番号で進む。ページ安定性は元parts・幾何・番号flag・全table継続を比較し、arena識別子を安定性の根拠にしない。source closureは参照された定義の非描画itemも含め元sourceを一回消費し、反復headerは元訪問済みsourceのartifactとして扱う。反復した最初のleafから脚注番号や元参照の注釈を再生成しない。

複数脚注の予約形式、19形式の定義内容、未参照の入れ子定義、本文／脚注双方の入れ子表を実driver PDFへ接続した。実ページの配置・source closureを含むrecord/workのexact/1不足も検証する。原ノ味無変更フォントの本文／脚注表fixtureは4ページPDFとなり、独立検査は元フォントメトリクスからbaselineと列位置を算出し、番号・リンクの一回生成とheader反復を確認した。合成フォント用本文48ptに対し原ノ味fixtureでは本文80ptを明示し、脚注48ptとseparatorは維持する。48pt入力のOversizeをエンジン修正で解消したとは扱わない。

統合148 tests、pagination 99 tests、独立384 PDFs（実driver 183件）/1,213ページ/2,771改変拒否が成功した。本文／脚注表専用検査は4 PDFs/12ページ/32改変拒否を確認した。従来332 PDFと原ノ味VMB table 2 PDFはbyte一致する。本節で小規模の脚注table実PDF成功を確認したが、残りの本文・ページ形式、元全巻、公開runner・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-definition-table-pages-design-14192)を参照する。


### 14.193. 異なる自然行高を持つセルの継続と原ノ味48pt入力（2026-09-10）

[ADR-0059](../adr/ADR-0059-book-2-natural-cell-continuations.md)に従い、元セルの分割禁止区間を統合した後、本文の共通境界の間隔が最大領域から反復header高を引いた容量へ収まるか検査する。収まらない区間があるbook-2 tableには、既存のセル別元sourceカーソルを準備する。最大容量に一致しても境界がkeepで閉じていれば同様とする。準備時に経路を確定し、発行済みカーソルやarenaを途中で切り替えない。共通境界で組める表と旧profileの経路は維持する。

異なるセルの行高が17ptと23ptの場合、各行は入るのに統合した禁止区間が行全体を覆い、従来は48ptの空ページでもOversizeになった。セル別継続では元の各セルを独立に消費し、caption・header・rowspan・keep・source closureと累積予算を共通処理で維持する。実行高を縮めず、未消費の行や参照を飛ばさない。元rowspanのsource順による行高配分も維持し、文字が終わった後に残る行領域は非描画の幾何としてページを消費する。

§14.192で80ptへ変更した原ノ味fixtureを残したうえで、元の本文48pt条件も成功させた。48pt版の本文は2ページへ分かれ、脚注の表は4ページへ継続する。通常表・入れ子・header・caption・rowspanの5形式とkeep／oversize拒否の2形式を検証し、実ページ配置とsource closureまでのrecord/work予算のexact/1不足を確認した。専用独立検査は元の宣言行高・フォントメトリクス・列幅・rowspan配分からPDF座標を求め、10 PDFs/30ページ/62改変拒否を確認した。本文／脚注表の検査も48ptと80pt両条件を含む6 PDFs/20ページ/48改変拒否へ拡張した。

統合149 tests、pagination 99 tests、旧production table 21 testsが成功した。独立396 PDFs（実driver 189件）/1,251ページ/2,851改変拒否が成功し、従来384 PDFと原ノ味VMB table 2 PDFはbyte一致する。残りの本文・ページ形式、元全巻、公開経路・manifest・管理ホスト・著者および人手受入は引き続き必要である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-natural-cell-continuations-design-14193)を参照する。


### 14.194. ページごとのマスター選択とPDFの用紙・移動先座標（2026-09-10）

[ADR-0060](../adr/ADR-0060-book-2-selected-page-masters.md)に従い、既存styleの選択優先順位を元wireのマスター選択と共有する。名前付き条件、先頭条件、偶奇条件の具体性、最後にsource_orderで比較し、偶奇は1始まりの物理ページ番号、firstは物理page index 0を使う。選択結果は元StyledBookV2Bodyとbase／advancedの元master identityを借用し、ruleと二分探索の訪問を同じwork予算へ課金する。選択だけでは配置・描画のreceiptを発行しない。

private driverは実際に選択した先頭masterの本文矩形から開始し、PDF assemblyは確定した各物理ページの選択を保持する。MediaBox・TrimBox・ページ全体のY変換にはそのページの寸法を使う。リンク矩形は参照元ページ、内部destination・name tree・outlineの座標は参照先ページの高さで変換する。最初のmasterの高さを全ページや他ページの移動先へ流用しない。本文・生成ページ番号の共通収束処理も維持する。

本文矩形が実際の行frameと異なる選択はPageFrameMismatchで拒否する。本節追加時点の本文組版は同じframeを使うため、異なる本文／脚注領域への再組版は未接続だった。同じ幅で高さ・縦位置が変わる領域は§14.195で接続した。名前付きruleの選択自体は検証したが、元本文のnamed-page遷移は引き続きPendingNamedPageとなる。脚注frame準備のsingle-master契約と、header/footer content・columnsの診断も維持する。本節はこれらのguardを外したという意味ではない。

先頭／偶数／通常ページで280×240、260×220、240×200ptを選ぶ3ページPDFを、合成フォントと原ノ味無変更フォントで検証した。非対称trim、前後の相互参照「3・1・2」、未選択master、同点時の後勝ち、名前付き条件、選択workのexact/1不足、page overflow、異なる本文幅の拒否を確認する。独立検査は元ruleから各masterを求め、用紙・stream変換・注釈・移動先を照合し、別ページのboxや誤った参照先高さの改変も拒否した。

統合151 tests、pagination 99 tests、style 14 testsとcompile-fail doc-testが成功した。独立403 PDFs（実driver 192件）/1,266ページ/2,929改変拒否、専用の4 PDFs/12ページ幾何検査が成功し、従来396 PDFと原ノ味VMB table 2 PDFはbyte一致する。新たに受理する未選択masterのfixtureにより、明示的な未対応入力は14から13となった。元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-selected-page-masters-design-14194)を参照する。


### 14.195. 本文・脚注の計測領域と実ページ領域の分離（2026-09-10）

[ADR-0061](../adr/ADR-0061-book-2-variable-page-frames.md)に従い、元StyledBookV2Bodyに結び付いたframe planへ、先頭・後続偶数・後続奇数ページの実矩形を保持する。元page limit内で到達可能な各条件のrule走査とmaster照会を累積workへ課金する。本文と脚注の計測にはそれぞれ最大高を使い、計測用矩形を物理ページのreceiptとして扱わない。異なるmasterから最大高を得る場合も、各ページの実矩形を別に保持する。

private driverはplanを明示的に渡す行収束経路を使い、行ownerは元styled sourceと計測本文の一致を検証する。ページ候補の列挙・評価と順位には選択した本文高を使い、脚注の同時予約には実際の本文／脚注矩形を使う。候補評価後は成功・失敗とも一時的なページ領域を戻し、個別部品APIへ状態を持ち越さない。選択ページに本文と宣言脚注矩形を残し、安定性照合・本文のY配置・最終PDFのsource master照合まで接続した。従来の部品APIのframe契約とsingle-master脚注検証は維持する。

表は最大領域で一度計測し、高さが変わる場合には最小の選択領域も用いて継続方式を準備する。脚注ではseparator帯を引いた容量を使う。大きいページで共通境界が使えても、小さいページに入らない統合区間があれば元セル別カーソルを保持する。途中でarenaや継続表現を交換しない。現在の空ページに不可分itemが入らなければ失敗し、後続の大きいページまで任意の空白ページを挿入したりsourceを飛ばしたりしない。

本文11行が先頭から1・2・3・2・3行へ分かれる5ページ、7行の脚注が実領域の下端へ揃って継続する場合、17pt／23ptの表が本文と脚注でそれぞれ3ページへ分かれる場合を検証した。計測・driver workのexact/1不足、本文／脚注のXまたは幅変更の拒否、8ptの先頭本文に16pt行を置けない場合の拒否も確認した。原ノ味無変更フォントは20pt行高と20／40／60pt領域で5ページを生成した。独立検査は元宣言とフォントメトリクスからbaseline、脚注番号幅、列位置、実領域下端、元source・注釈数を算出する。

統合155 tests、pagination 99 testsが成功した。独立413 PDFs（実driver 197件）/1,308ページ/3,015改変拒否、可変領域専用10 PDFs/42ページ/76改変拒否が成功し、従来403 PDFと原ノ味VMB table 2 PDFはbyte一致する。本文／脚注の横位置・幅の変更にはHorizontalReflowを返し、名前付き本文遷移、柱・footer content、段組の診断も維持する。元全巻、公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-variable-page-frames-design-14195)を参照する。


### 14.196. 元本文の名前付きページ指定と継続（2026-09-10）

[ADR-0062](../adr/ADR-0062-book-2-named-page-scopes.md)に従い、元flowのBegin/Endとtyped page指定から各本文領域の使用ページ名を求める。autoの子は外側の名前付き領域を使い、内側の明示名はその領域のEndまで有効とする。終了後は外側へ戻る。元ownerと名前の対応、各名前の先頭・偶数・奇数masterを保持し、元styled sourceに結び付いた一つのplanを行収束の各passが借用する。source走査、名前照会、master選択、sortは累積workへ課金し、名前・対応recordと文字列bytesをcommandの残り予算から確保する。

名前が変わる本文境界でページ候補を止め、その境界を順位付けの終端として扱う。先頭の名前付き本文は物理page 0で選び、前に合成空白を挿入しない。keep_with_nextと名前変更が衝突すれば診断する。表の継続には元root tableの名前を保持し、本文が完了して脚注だけが残るページにも直前の名前を渡す。名前付き領域内の先頭・連続・末尾の元改ページは従来のsource進行を維持する。選択名・実矩形を安定性照合とPDFへ保持し、PDF master照会には物理page indexと実際の名前を渡す。生成ページ番号のsource-to-PDF収束も維持する。

表のpage指定には後継専用のstyle検証・cascade経路を追加した。旧table-1のauto限定検証器と利用経路は維持し、元versionに結び付いたprivate rule ownerが使い分ける。一つの並列表内でcell／入れ子表が別名を要求する場合と、脚注定義内の名前付き内容にはowner付き診断を残す。独立した改ページnode自身への明示page指定もNamedPageBreakで拒否し、指定を黙って捨てない。外側の名前付き領域に通常の改ページnodeを置く場合は対応する。

通常→appendix→通常の前後参照「3・1・2」、appendix→short→appendix→通常の入れ子、名前付き領域の先頭／連続／末尾の改ページによる空ページ、本文表の2ページ継続後の通常本文、本文完了後も同じ名前で進む脚注4ページを検証した。planのrecord・名前bytes・workと実driver workのexact/1不足、keep衝突・並列表内の別名・名前付き脚注定義・独立改ページ指定の拒否も確認する。原ノ味無変更フォントでは参照3ページと入れ子4ページを生成した。

統合160 tests、pagination 99 tests、style 15 testsとcompile-fail doc-testが成功した。独立427 PDFs（実driver 204件）/1,360ページ/3,149改変拒否、名前付きページ専用14 PDFs/52ページ/98改変拒否が成功し、従来413 PDFと原ノ味VMB table 2 PDFはbyte一致する。専用検査は元sourceの領域構造、宣言master、フォントメトリクス、コンテナの字下げ、列位置、脚注下端、注釈と元文字列を照合した。横位置・幅の変更、残る名前付き改ページ形式、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-named-page-scopes-design-14196)を参照する。


### 14.197. 改ページノード自身の名前指定と非描画ページ（2026-09-10）

[ADR-0063](../adr/ADR-0063-book-2-explicit-break-page-names.md)に従い、後継の元source flowへ明示的な改ページ名と元node identityを保持した。owner順のregistryを元flowの再検証とfingerprintへ含め、名前bytesをsource text予算へ課金する。元page planは同時に保持するregistryのrecord・bytes、sortとowner照会もcommandの累積予算へ含める。旧flowではregistryを作らず、名前付き改ページがない場合のencodingを維持する。

改ページ自身の名前は、その元nodeを消費するページを選ぶ。End後は外側の使用名へ戻り、文書直下なら後続本文は通常の名前へ戻る。他の名前付き領域と同じく、名前の変更と明示改ページは別の境界である。名前の変更によって直前のページを閉じ、その改ページを一度消費して次ページへ進む。先頭・連続・末尾の改ページを維持し、末尾の空ページには最後に消費した領域名を保持する。元sourceを合成nodeへ置き換えない。

既存の本文選択は直後の表外改ページを現在のページへ取り込んでいたため、その使用名が現在のページと一致する場合に限定した。異なる場合は元nodeを未消費のまま次の名前付き領域へ残す。keep矛盾、本文／表／脚注の元sourceの一回消費と継続を維持する。名前planがない部品経路、名前付き脚注定義、並列表内の異なる名前はowner付き診断を保つ。

入れ子領域と文書直下の両方で、名前付き改ページ3個、通常改ページ1個と本文2個を元順序で検証した。名前変更を含む7ページへ進み、本文は2・5ページ、他は元境界による空ページとなる。通常ページへ戻る場合と外側appendixへ戻る場合を区別し、原ノ味無変更フォントでも同じ元source順と実masterを照合した。元ownerの保持、flow再検証、record・bytes・workのexact/1不足、並列表／脚注定義の名前付き改ページ拒否も確認した。

統合160 tests、pagination 99 tests、旧表21 testsと旧名前付き段落policy 1 testが成功した。独立435 PDFs（実driver 208件）/1,416ページ/3,213改変拒否、名前付きページ専用22 PDFs/108ページ/154改変拒否が成功し、従来427 PDFと原ノ味VMB table 2 PDFはbyte一致する。横位置・幅変更時の再組版、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-explicit-break-page-names-design-14197)を参照する。


### 14.198. ページ別の本文・脚注の横位置（2026-09-10）

[ADR-0064](../adr/ADR-0064-book-2-horizontal-page-origins.md)に従い、幅が同じ本文／脚注領域について、元page planへ異なる横位置を保持できるようにした。名前未指定時の先頭frameの計測原点を維持し、物理ページの選択原点との差を実配置へ適用する。本文と脚注の差分は独立に求め、脚注領域が本文より左にある場合も扱う。幅変更時のsource-awareな再組版は別に必要であり、HorizontalReflow診断を維持する。

元の通常fragment、表セル、反復header、図版viewportへ対応する領域の差分を一度だけ加える。リスト番号は結び付いた元fragmentの本文／定義区分を使い、脚注番号は定義領域と一緒に移動する。separatorは既に選択済み脚注矩形から作られるため二重移動しない。数式番号は移動後の親boundsとviewportから配置する。座標計算はchecked arithmeticを使い、移動するfragment／markerの訪問を共通workへ課金する。差分が0なら従来の配置経路を維持する。

source closureのblock viewport照合も、元計測位置に選択ページの差分を加えた値と比較する。位置検査を外さず、元sourceの一回消費、反復属性、安定性、最終PDFの描画・注釈・移動先へ実配置を渡す。行高・行幅・列幅や元source順を変更しない。

通常本文、脚注、本文／脚注の異なる自然行高の表、名前付き前後参照と入れ子、反復header・リスト・脚注、番号付きvector blockについて、元位置と移動後の組を検証した。本文・脚注の宣言原点を引いた配置は、元owner、source/item、bounds、viewport、各番号とseparatorまで一致する。実driver workのexact/1不足も確認した。原ノ味無変更フォントでは脚注5ページと名前付き参照3ページを両条件で生成し、本文と脚注が異なる横位置を持つ3ページ目を描画して確認した。

統合162 tests、pagination 99 tests、旧表21 testsと旧名前付き段落policy 1 testが成功した。独立475 PDFs（実driver 228件）/1,608ページ/3,589改変拒否、横位置専用40 PDFs/192ページ/120改変拒否が成功し、従来435 PDFと原ノ味VMB table 2 PDFはbyte一致する。専用PDF検査は元masterの差分から実文字・注釈・名前付き移動先を照合する。幅変更時の再組版、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-horizontal-page-origins-design-14198)を参照する。


### 14.199. 元の行開始位置に結び付いた幅指定（2026-09-10）

[ADR-0065](../adr/ADR-0065-book-2-source-start-inline-widths.md)に従い、元の準備済み段落itemへ各logical unit開始位置の幅を結び付ける型を追加した。空段落は1要素、通常段落は元unit数と同じ要素数を要求する。cluster内部の到達不能位置も保持し、全指定をfingerprintへ含める。行番号は改行の再選択によって変わるため、候補の元開始位置から幅を選ぶ。候補途中で幅が変わっても、それ自体を強制改行として扱わない。

固定幅と可変幅で共通の最小demerit探索を使い、元cluster、明示改行、atomic item、負の描画原点の補正を維持する。選択行に実際の幅を残し、狭い開始位置の元sourceを飛ばして後続の広い場所へ進むことを認めない。固定幅のcanonical bytesは維持し、可変幅には別algorithmと全幅指定を含める。幅走査を既存workへ、profileと幅recordをcommandの残りrecordへ課金する。

Book-2の行投影はprofileが同じ内容の別準備物ではなく、今回の元段落itemそのものを借用することを検証する。元字形への参照と元source spanを維持して選択行を作り、選択幅が指定envelopeを超えれば拒否する。出力は物理frameを持たず、実ページの選択結果として扱わない。幅変更の実ページfeedbackと再shapingへの接続は残っており、page planのHorizontalReflow診断は維持する。

729通りの幅指定を6 unitの全32分割と比較し、最小コスト、元範囲の連続、workと選択行数のexact/1不足を確認した。不可分cluster、到達不能幅のfingerprint、空段落、vector overhangと元明示改行も検証する。元の実shapingを通る合成フォントと原ノ味無変更フォントでは、固定幅と可変幅で異なる改行を選び、元字形identity、source span、残りrecordとwork、別準備物の拒否を確認した。

行分割50 unit testsとUnicode適合1 test、統合164 tests、pagination 99 tests、旧表21 testsと旧名前付き段落policy 1 testが成功した。独立475 PDFs（実driver 228件）/1,608ページ/3,589改変拒否が成功し、既存475 PDFと原ノ味VMB table 2 PDFはbyte一致する。今回は新しい可変幅PDFを生成したとは扱わない。幅変更時の実ページ再組版、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-source-start-inline-widths-design-14199)を参照する。


### 14.200. 元flowを保持した幅の再bindingと行収束（2026-09-10）

[ADR-0066](../adr/ADR-0066-book-2-source-width-reshape.md)に従い、可変幅指定を準備済み字形とは独立した元Book-2 flowへ結び付ける型を追加した。各段落の幅sliceを借用し、実shaping後の新しいitemへ毎回bindingする。同じ内容でも別flowには使えず、ページ番号などの生成文字列を更新してflowを作り直した場合は新しい指定が必要になる。constructorは段落数を確認し、要素数とsource対応は各投影で検証する。

元flowを順に走査し、文字は元scalar、vectorとnative数式は元ownerとspan、改行は元owner・span・kindを新itemのlogical unitと照合する。アンカーとinline container境界は幅slotを消費しない。空段落は1 slotと非描画1行を維持する。元unitの欠落・追加・順序変更を許さず、旧itemのpointerだけを付け替えることもしない。binding vectorの確保前に、以前のrecordと保持中のnative computationを含めて容量を検査する。binding／幅recordは共通投影が一度課金し、段落・site・文字の訪問を行探索と同じworkへ含める。

実際の行収束ループへ接続し、初回と各再shaping後に同じ元flowから幅を結び直す。選択した元byte境界を次のshaping contextへ渡し、実選択fingerprintの一致で安定を判断する。初回・再選択のworkとreshape passを累積し、元native computationも再利用する。既存の固定幅callerは従来経路を使う。保持するframeは計測envelopeであり、選択幅がその幅を超えれば拒否する。

合成フォント、原ノ味無変更フォント、1字形に2 scalarを含む「か＋結合濁点」、native数式、vector／math vector、明示soft／hard break、アンカー、空段落、表・リスト・脚注の生成番号を検証した。全段落指定と一部None指定の固定幅fallback、元字形pointer、source順、record・work・reshape passのexact/1不足も確認する。原ノ味の通常文字と結合文字は実shapingを2 pass行って安定した。

統合168 tests、layout 69 testsとdoc-test 1件、pagination 99 tests、旧表21 testsと旧名前付き段落policy 1 testが成功した。独立475 PDFs（実driver 228件）/1,608ページ/3,589改変拒否が成功し、既存475 PDFと原ノ味VMB table 2 PDFはbyte一致する。実ページから幅指定を作るfeedbackはまだ接続しておらず、HorizontalReflow診断を維持する。本文／脚注／表の物理幅配置、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-source-width-reshape-design-14200)を参照する。


### 14.201. 選択ページから元段落への幅feedback（2026-09-10）

[ADR-0067](../adr/ADR-0067-book-2-paragraph-page-width-feedback.md)に従い、発行元searchの実ページ配置とsource closureから、次の再組版に使う段落幅を作る処理を追加した。各行の元logical unit範囲に、計測envelopeと選択済み本文／脚注領域の幅差から求めた幅を割り当てる。空段落は非描画1行に対応する1 slotを使う。元段落ownerと生成文字列を含むflow fingerprintを保持し、同じ内容の再生成flowで候補を再利用できることを確認する。新しいshapingでは§14.200の元flow検証とitemへの再bindingを行う。

反復header／captionと元の意味的な出現を別に記録し、反復によってsourceを二度消費しない。同じ元unitへ異なる幅が要求された場合は拒否する。観測した段落は全unitの意味的な出現を要求し、未参照定義全体は明示的な未観測として計測幅を保持する。本文の幅差をfraction列の再計測に見立てず、表を含むflowのroot幅が変わる場合はtable_width_reflowを返す。結果と一時coverage recordを確保前に課金し、段落・ページ・fragment・unitの走査をsearchの累積workへ含める。

本文collectorの右寄せ・中央寄せは、段落envelopeの幅ではなく各選択行の幅から余白を計算するよう修正した。狭い候補行をenvelopeの右端へ移してしまうことを防ぐ。選択幅がenvelopeを超える場合は拒否し、空行の正のbounds幅にも選択行幅を使う。固定幅では従来の計算結果を維持する。

始端・終端・中央揃えについて、実shaping→改ページ→配置→source closure→幅feedback→新しいshaping→再改ページを検証した。合成フォント本文は4→1ページ、原ノ味無変更フォント本文は4→2ページとなり、空段落は1ページを維持した。9件のsource proofへ幅・work・recordを記録する。反復する表header、リスト、要求された脚注と未参照定義も検証し、別searchのclosureとrecord／workの1不足を拒否した。

統合171 tests、pagination 99 tests、旧表21 testsと旧名前付き段落policy 1 testが成功した。独立475 PDFs（実driver 228件）/1,608ページ/3,589改変拒否が成功し、既存475 PDFと原ノ味VMB table 2 PDFはbyte一致する。今回の新規feedback試験は部品の実ページ配置を検証しており、新しい可変物理幅PDFの生成成功とは扱わない。幅が異なるmasterの受理、private driverでの幅・生成番号の同時収束、本文／脚注／表・blockの幅配置、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は未完である。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-paragraph-page-width-feedback-design-14201)を参照する。


### 14.202. 可変幅の本文・脚注と生成ページ番号の収束（2026-09-10）

[ADR-0068](../adr/ADR-0068-book-2-variable-page-widths.md)に従い、元flowを使うpage planに異なる本文／脚注幅を保持し、private source-to-PDF driverへ幅feedbackを接続した。独立した最大幅・最大高は計測用とし、実際の先頭・偶奇・名前付きmasterの矩形を保持する。従来のstrict部品APIは維持し、表・図版・block数式を含む幅変更は各要素の再計測が必要としてHorizontalReflowを返す。

実shaping、改ページ、配置、source closureの後で元段落の開始位置へ幅を返し、各passで新itemへbindingする。選択行の実幅一致に加え、到達不能位置を含む全幅指定の一致を要求する。生成ページ番号で元flowが変われば幅候補を破棄し、幅変更時にはPDF安定性を再検証する。参照を持つ入力には従来どおり連続した完全PDFの一致を要求する。PDF前のmath finalizationも選択幅と物理領域を独立に照合し、部品callerによる暫定幅の描画を拒否する。

原ノ味脚注では、将来の幅変更によって直前の最小demerit改行を取り消す2状態の循環を再現した。元source・owner・全幅指定のdigestを定数個保持するBrent検出を追加し、循環時にはsource-closedな元行末を制約として保持する。以後は必要に応じて分割を追加できるが、その行末をまたぐ再結合は行わない。生成文字列が変わればこの制約も破棄する。

共通の行分割kernelが、保持した境界の単調増加・終端・合法性と新itemのcluster不可分性を検証する。空段落は終端0を一つ保持し、元のmandatory breakとatomic itemのoverhangも維持する。制約を元sourceの強制改行として挿入しない。別algorithm `typaxis.refined-width-inline-break/1` と全幅・全保持行末をcanonical bytesへ含め、制約内で最小demeritを選ぶ。固定sourceで分割は有限だが、全ページと改行の無制約大域最適性や任意の予算内の成功は主張しない。recordは確保前、走査と再試行は累積work・passへ課金する。

合成フォントと原ノ味無変更フォントで、本文・脚注・相互参照の計6入力を検証した。原ノ味本文は3ページ、脚注は8ページとなり、脚注は12回の幅feedback中2回のrefinementで収束する。原ノ味参照は11ページで最終番号8・1・5を描画する。全ケースでworkのexact成功と1不足拒否を確認した。648通りの幅／保持境界を全合法分割の最小コストと比較し、不正境界、cluster、空段落、元改行、vector overhang、部品finalizationの拒否も検証する。

統合175 tests、行分割52 testsとUnicode適合1 test、独立487 PDFs（実driver 234件）/1,672ページ/3,721改変拒否が成功した。可変幅専用12 PDFs/64ページ/68改変拒否は、元sfntのcmap・hmtx・hheaと宣言masterから本文／脚注の幅、字幅、中央揃え、baseline、元文字列、脚注番号、最終ページ参照を照合した。既存475 PDFと原ノ味VMB table 2 PDFはbyte一致する。追加の回帰検証と証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-variable-page-widths-design-14202)を参照する。

幅変更を伴う表・block要素、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻、公開経路・manifest・管理ホスト・著者および人手受入は未完である。今回の成果はprivate stagingの本文・脚注・参照PDFであり、設計全体の完了とは扱わない。


### 14.203. 図版・block数式の物理幅への再binding（2026-09-10）

[ADR-0069](../adr/ADR-0069-book-2-block-page-widths.md)に従い、元flowのblock ownerへ親枠の幅を結び付ける候補を追加した。元source event順に通常図版、vector図版、native display数式、precomposed display数式を照合し、別flow、欠落・余分・並べ替え・重複したownerと計測envelopeを超える幅を拒否する。frameを持たない段落投影へblock指定を渡した場合も拒否し、指定を黙って捨てない。

各実shaping passでblock自身の親枠へ候補幅を適用し、元の計測親枠を別に保持する。元の横位置と段落envelopeは変更せず、全block指定をframe fingerprintへ含める。図版と数式の宣言寸法、SVG scale、元字形・native computation・resource identityを保持し、字下げ、中央／右揃え、数式番号の位置を新しい親枠で再計測する。captionは既存の段落幅feedbackを使う。数式番号は現行契約の横並びとminimum gapを維持し、狭すぎる場合に番号を下へ移動したり図版を縮小したりしない。

source-closedな実ページ配置から、元計測親枠と選択された本文／脚注領域の幅差によって各blockの目標幅を求める。元source順の全指定をdriverの幅比較・循環digestへ含め、次passでblockを再計測する。意味的な出現と反復を区別し、異なる反復幅や意味sourceの二重消費を拒否する。未参照定義は元計測幅を保持する。幅を変える表は列・cellの再計測が必要なため、引き続きHorizontalReflowを返す。

最終math処理でもblock親枠と実ページ幅を独立に照合する。可変page planがなくても、明示的なblock幅候補を持つ部品callerにはこの検証を適用する。元親枠の保持・候補・coverage recordは確保前に課金し、source訪問、幅照会・sort・hash・比較と物理検証を既存の累積workへ含める。

始端・中央・終端揃えで再shaping後のframe・viewportと元resourceのidentity、workのexact/1不足、不正binding、番号の幅不足、暫定幅によるfinalization拒否を確認した。合成フォントではvector図版とcaption・番号付き数式、中央揃えnative数式、PNG、JPEG、通常SVGの本文／脚注10ケース、無変更原ノ味では図版4形式の本文／脚注8ケースを実PDFへ接続した。native数式の複製は入力中の独立した範囲とidentity mappingを持ち、math domain重複のvalidatorを維持する。

統合178 testsと、可変block幅専用36 PDFs/74ページ/72改変拒否が成功した。専用検査は宣言masterと元fontの字幅から図版・番号の横位置、固定寸法・SVG scaleを照合し、native数式は実glyphの幅と検証済み埋込advanceを使って中央揃えを照合する。数式内部の大域最適性を別に証明したとは扱わない。既存487 PDFと原ノ味VMB table 2 PDFはbyte一致する。全体の独立検査と回帰証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-block-page-widths-design-14203)を参照する。

幅変更を伴う表、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻、公開経路・manifest・管理ホスト・著者および人手受入は未完である。今回の成果はprivate stagingの固定寸法blockの再配置であり、設計全体の完了とは扱わない。


### 14.204. 元の表階層に結び付いた幅候補と再計測（2026-09-10）

[ADR-0070](../adr/ADR-0070-book-2-root-table-width-frames.md)に従い、元flowのroot table全件へsource event順で親枠幅を指定する借用候補を追加した。別flow、欠落・余分・並べ替え・重複、入れ子表や表以外のowner、元親枠より広い指定を拒否する。指定が空なら既存の計測を維持する。frameなしの段落投影へ渡した場合も拒否し、表幅を黙って捨てない。

既存のframe走査と固定／比率列の計算を共用し、root tableの元字下げと列幅計算の前に候補親幅を適用する。入れ子表は新しい結合セルの幅から再計測する。caption、header、cell、内部コンテナ・list・段落も元階層から計算し直し、rowspanのsource格子、元resource・字形・spanを維持する。各passの実shapingへ新しい段落幅を渡し、元文字列を再組版する。比率列の丸めと末尾比率列への正負の端数配分は既存kernelを使う。

元の計測projectionを別に保持し、表・段落・owner領域の元枠を参照できるようにする。二つ目のprojectionを確保する前に、元projectionの保持分と新しいprojectionの全record上限を検証する。追加の走査・列計算・owner照会・fingerprint encodingには、元record上限の64倍とsource event数から求める照会深さによる保守的なwork上限を前払いする。各再shaping passで残りの累積workから引き、専用algorithmと実枠をfingerprintへ含める。

これは物理ページ幅feedbackの前提となる部品の実再計測である。幅が変わるmaster上の表は引き続きHorizontalReflowを返す。明示的な表幅候補を持つ部品callerについても、段落幅feedbackと最終math処理はtable_width_reflow診断を返す。可変page planがなくても暫定表幅からPDFを発行しない。異なる幅のページへ継続する表の列・セル対応、実ページ選択との収束は次の接続課題である。

合成フォントと無変更原ノ味の本文／脚注で、二つのroot table、入れ子、caption、header、colspan・rowspanを再組版した。元計測枠と本文／脚注のrootを保持し、段落16件の幅変更と実際の行数増加、±1 raw単位の端数、列位置・字下げ、元文字列・glyph identityを照合した。work・reshape passのexact成功と1不足拒否、不正bindingと暫定finalization拒否も確認した。統合回帰と証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-root-table-width-frames-design-14204)を参照する。

表の可変物理幅PDF、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻、公開経路・manifest・管理ホスト・著者および人手受入は未完である。本節は設計全体の完了や新しい可変幅表PDFの成功を意味しない。


### 14.205. 実ページ幅から元の表の再計測・PDFへの接続（2026-09-10）

[ADR-0071](../adr/ADR-0071-book-2-table-page-widths.md)に従い、source closure済みの本文／脚注の選択table区間から、元root tableの親枠幅を求める。描画leafだけで判定せず、高さ0のtable区間も候補として訪問する。元計測親枠と実際の本文／脚注幅との差を使い、元root ownerと定義のdemandを照合する。未参照定義は計測幅を保持する。同じroot tableが選ばれた全区間では親幅の一致を要求し、継続先で幅が異なる場合はtable_continuation_width_reflowを返す。

元順序のroot幅と一致判定を既存の幅feedbackへ保持し、新しいflowへのbinding、driverの全指定比較・循環digest、次の実shapingとページ再選択へ接続した。表内の段落は元global item範囲から所属を求め、再計測したcaption／cellの枠幅を使う。本文の幅差を比率列へ加算せず、前passの段落幅で新しい列を上書きしない。表内段落へ段落専用の保持行末refinementを適用せず、表幅の循環や未収束は既存の累積work・pass上限で診断する。生成ページ番号が変わった場合の候補破棄と最終PDFの連続一致を維持する。

最終math処理でもroot表幅を実選択から求め直し、不一致ならWidthMismatchとする。明示的な表候補だけでなく、表を含む可変幅planで未再計測の部品callerにも適用する。表内の選択行は再計測した枠と、表外の行は実ページ領域と照合する。元source、格子、resource identity、反復と一回消費の検証を維持する。reflow用page planは表をこの実検証へ渡せるようになり、厳密な部品planの異幅拒否は維持する。

表内blockへの親幅feedbackは引き続きtable_block_width_reflowとして診断し、旧親枠の指定を新しい列へ流用しない。target／coverageのrecordは確保前に課金し、table区間・leaf訪問、hash・比較と最終幅照合を共通workへ含める。計測枠照会がblock registryから元region mapへ進む場合の探索深さも課金するよう修正した。

合成フォントと無変更原ノ味で、本文表3個、脚注定義の表3個、先頭／偶数の同じ幅へ継続する表と後続の広い表を実PDFへ接続した。各ケースは3ページとなり、入れ子、caption、header、colspan・rowspan、元字形・文字列を保持する。独立検査はPDF ParentTreeから元段落を区別し、各caption・cell・入れ子段落の実文字位置を宣言master、固定／比率列、字下げと元font字幅から照合する。脚注番号は数字ごとの元advanceを使って右揃えも検証する。work exact／1不足、未再計測のfinalization拒否、継続先の異幅拒否を含む回帰と証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-page-widths-design-14205)を参照する。

同じ表が異なる幅へ継続する場合、表内block、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻、公開経路・manifest・管理ホスト・著者および人手受入は未完である。本節はprivate stagingの表PDFへの接続であり、設計全体の完了とは扱わない。


### 14.206. 表内図版・block数式の再計測した親枠への追従（2026-09-10）

[ADR-0072](../adr/ADR-0072-book-2-table-block-widths.md)に従い、元source幅指定へ表内blockの親枠継承を明示するmodeを追加した。driverはroot表幅と全block owner指定を渡し、このmodeを使用する。元event走査でroot表の内外と階層深さを定数状態で追跡し、表内blockは再計測したcell／コンテナの親幅を使う。表外blockは従来どおり指定幅を使う。全blockの元owner順序・欠落・余分・重複・別flow検証を維持し、旧幅が新しいセル枠を上書きしない。

元の最大計測枠、表階層から計算したblock継承枠、明示override後の実枠を区別する。block override前の枠は既存の保持recordへ残し、fingerprintには実際に使った幅を含める。部品callerの明示幅modeは維持し、継承枠と異なる明示幅では最終出力を拒否する。

実ページの元item範囲から表内blockの所属を照合し、block幅feedbackは再計測した継承枠を返す。root表の物理幅は別に選択区間から照合し、不一致なら次passの表再計測へ進む。最終math処理でもroot表幅とblock継承枠の両方を検証する。表内blockの従来のtable_block_width_reflow診断をこの実処理へ置き換え、source closureと元resource identityの検証は維持する。

図版寸法・SVG scale、native computation・元字形、captionと数式番号のminimum gapを保持し、新しい親枠で配置を再計測する。元event訪問とblock保持recordの課金を共用し、深さはchecked arithmeticで計算する。継承枠のregistry照会とregion mapへのfallbackの探索深さを共通workへ課金する。

合成フォントではvector図版・caption・番号付き数式、中央揃えnative数式、PNG、JPEG、通常SVGの本文／脚注10ケース、無変更原ノ味では図版4形式の本文／脚注8ケースを入れ子表PDFへ接続した。二つの20pt固定列と親の結合cellを保持し、元計測枠から選択幅へ再組版する。専用独立検査は元列宣言から40ptの親位置差を求め、実画像・数式・番号の位置、固定寸法、元font字幅とnative glyph extentsを照合する。部品の明示／継承mode、work exact／1不足、明示幅不一致のfinalization拒否を含む回帰証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-block-widths-design-14206)を参照する。

同じ表が異なる幅へ継続する場合、残る並列表・定義内の名前指定、柱・footer content、段組、元全巻、公開経路・manifest・管理ホスト・著者および人手受入は未完である。本節はprivate stagingの表内block PDFへの接続であり、設計全体の完了とは扱わない。


### 14.207. 異幅の表継続を元source位置で追跡する観測（2026-09-10）

[ADR-0073](../adr/ADR-0073-book-2-table-width-occurrences.md)に従い、source closure済みの実ページ選択から、表の継続区間ごとに元root owner、ページ、脚注定義の有無と親枠幅を保持する観測APIを追加した。発行元searchと元flowの同一性を要求し、元計測親枠と実本文／脚注領域の差から幅を計算する。同じ表の幅が区間ごとに異なっても、一つの幅へまとめずに保持する。描画leafを持たない高さ0の表区間も対象とする。

各段落は元logical unitの範囲で記録し、blockと強制改ページは元ownerで記録する。unitは元scalar・atomic inline itemであり、UTF-8 byte位置ではない。現在の行番号やleaf番号を次のshapingへ流用しない。意味的な一回消費と反復header／captionを区別し、反復にも元位置を残す。元flow fingerprintと全区間・source piece・反復flagを専用digestへ含める。生成番号を含む元flowの内容比較は再bindingの補助であり、別発行元への許可を与えない。

区間数と各区間のpiece数を先に数え、必要recordを検査してからvectorを一度確保する。件数の加算はchecked arithmeticを使い、事前走査、選択区間・leaf訪問とhash計算は共通workへ課金する。1件ずつの再確保による二乗時間のコピーを避け、累積work／recordのexact成功と1不足拒否を検証する。

本処理は異幅継続の再計測に必要な観測であり、物理PDFへの接続は未完である。既存のroot単位の幅feedbackと最終math処理は、引き続きtable_continuation_width_reflowで異幅継続を拒否する。次に元位置ごとのcell幅・横位置・反復headerを再計測し、ページ再選択と収束させる必要がある。検証結果と回帰証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-width-occurrences-design-14207)を参照する。

残る並列表・定義内の名前指定、柱・footer content、段組、元全巻、公開経路・manifest・管理ホスト・著者および人手受入も未完であり、設計全体の完了とは扱わない。


### 14.208. 継続区間ごとの表階層からセル幅・横位置を再計測（2026-09-10）

[ADR-0074](../adr/ADR-0074-book-2-table-occurrence-frames.md)に従い、元root表を指定した親幅で投影し直す処理を追加した。元source event順にroot指定を組み立て、対象rootだけを置き換えて既存のframe・列幅計算へ渡す。他のrootは元の最大計測幅を使い、前passの別root候補を混ぜない。入れ子表は再計測した結合セルから幅を継承し、固定／比率列、丸め、端数配分と元の字下げを共通kernelで計算する。元frameは上書きしない。

結果は入力frameそのものへ結び付き、元owner、段落枠、block親枠と表階層を保持する。別frameによる検証、非root・存在しない・入れ子owner、元親枠を超える幅、固定列を収められない幅と予算不足を拒否する。同じ実枠となる投影でも対象rootが異なればfingerprintで区別する。結果は再組版用の暫定geometryであり、実ページ・PDFの許可ではない。

元projectionの全record上限にroot指定vectorと結果の上限を加え、確保前に検査する。その64倍とsource eventの照会深さから保守的なwork上限を前払いする。paginationは試行前に共通record／workへ課金し、失敗した再計測も予算を返却しない。今回は区間ごとに一時的な全階層projectionを作る実装であり、元全巻での性能受入を済ませたとは扱わない。

source closure済みの各継続区間の実親幅で再計測し、元logical unit範囲へ段落枠を、元block ownerへ親枠を返すAPIを追加した。反復header／captionは各区間の枠を保持し、意味sourceを重複消費しない。強制改ページは枠を持たず、高さ0の表区間も残す。再計測した横位置・幅は専用digestへ含め、従来の観測のみのAPIと区別する。

合成フォントと無変更原ノ味の本文／脚注20ケースでは、元の等比率2列から独立に計算した横位置と幅を照合した。段落途中の分割、反復header、caption内の強制改ページ、空表、work／record exact成功と1不足拒否を検証する。別試験では入れ子、固定列、colspan・rowspanと正負1 rawの端数を確認し、対象外rootの計測枠が保たれることと不正指定の拒否を検証した。表内blockは部品の明示overrideと区別して再計測した継承枠を返す。統合回帰と証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-occurrence-frames-design-14208)を参照する。

元位置ごとの枠を実shapingと配置へ結び付け、反復headerを含めてページ選択と収束させる処理は残る。異幅継続のtable_continuation_width_reflow拒否は維持する。残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入も未完である。


### 14.209. 元unitの横位置を実shaping・本文配置へ接続（2026-09-10）

[ADR-0075](../adr/ADR-0075-book-2-source-unit-starts.md)に従い、元flowの幅指定へlogical unitごとの横位置を組み合わせる候補を追加した。横位置を持つ段落には同じunit数の幅指定を要求し、空段落は1 slotを使う。別flow、要素数の不一致、幅指定の欠落、元本文または脚注本文のenvelope外へ出る指定を拒否する。frameを持たない組版へ渡しても黙って破棄しない。

各実shaping passで元sourceのscalar・atomic item・改行対応を既存のbindingで検証し、横位置を新しいframeへ保持する。現在の行番号を次passへ流用せず、選択行の元開始unitから横位置を取る。元の段落envelopeと階層を維持し、候補の有無・owner・全横位置と幅を専用fingerprintへ含める。候補がない既存callerのencodingは変えない。

保持する全paragraph／unit recordを確保前に検査し、元event・unit訪問をtable／block再計測・行探索と同じ累積workへ課金する。各profileは一度確保してコピーする。脚注のenvelopeは元の生成markerを除いた本文枠を使い、横位置は計測本文のxを基準とする。現在は行頭として到達できないunitも保持・検証する。

本文collectorは選択行の元開始unitに対応する横位置を使い、その行の選択幅による余白から始端・中央・終端揃えを計算する。既存の本文／脚注の物理ページ変換で横位置を維持し、元の字形・span・source順と一回消費を保つ。最終math処理は横位置候補がある場合に幅変更なしのplanでも独立検証を行い、投影済み元段落の横位置と一致しなければ拒否する。同じ幅のまま横位置だけをずらしてもPDFの許可にはならない。

合成フォントと無変更原ノ味の本文／脚注で、§14.208の区間別frameを元unitへ割り当て、実shaping・行選択・本文収集・ページ選択・配置・source closureを通した。選択幅、字形identity、3種の揃えとページ変換後の横位置を照合する。幅変更が必ず行数変更になるとは扱わず、原ノ味本文は行数を保ったまま狭い行と新しい横位置を使う。別試験では正しい横位置を受理し、同じ幅で1 rawだけずらした横位置、別flow、不正な長さ・欠落・envelope外指定を拒否した。work exact／1不足と保持recordの増分を含む回帰証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-source-unit-starts-design-14209)を参照する。

private driverでの幅・位置feedbackの同時収束と反復headerの区間別layoutは残り、異幅継続のtable_continuation_width_reflow拒否は維持する。残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入も未完である。


### 14.210. 異幅で継続する表段落の幅・横位置feedbackと実PDF（2026-09-10）

[ADR-0076](../adr/ADR-0076-book-2-table-source-profiles.md)に従い、source closure済みの表の継続区間を元logical unitごとの幅・横位置へ変換し、private driverの実shaping・改ページ・PDF収束へ接続した。各区間の親幅で元表階層を再計測し、元ownerとunit範囲から現在の選択行を照合する。行番号やleaf番号を次passへ保持しない。高さ0のroot表と本文／参照された脚注定義のroot coverageを検証し、観測した元unitの意味的な一回消費を要求する。

元source幅指定と保持frameに区間別frameの明示modeを追加し、対応する横位置profileを必須とした。幅と横位置、modeを候補比較・循環digestとframe fingerprintへ含める。driverはまず従来のroot単位feedbackを使い、厳密にtable_continuation_width_reflowが返った場合だけ、同じsearchで区間別profileを求める。失敗した試行のwork／recordも累積予算へ残す。以後はこのmodeを保持し、新しい元flowに幅と横位置を再bindingする。表の最大source envelopeを維持し、一つのroot幅で区間別の幅を上書きしない。

元unitの横位置と選択幅から始端・中央・終端揃えを求め、本文／脚注の実ページ変換へ渡す。幅候補が循環した場合は既存の元source行末保持を表段落にも適用し、次passで細分化を許可しつつ保持境界をまたいだ再結合を防ぐ。生成ページ番号が変わった場合の候補破棄、共通のwork・record・reshape・page pass上限、連続した最終PDF候補の一致を維持する。横位置のcopy、元owner照会、全指定比較、区間再投影を課金する。

最終math処理は明示modeの場合に、root単位の単一幅検査を区間ごとの独立した再計測へ置き換える。元cell／captionの枠と、実選択行の幅・横位置が一致することを確認する。表外の物理枠検査、source closure、元resource identityを維持する。幅変更のないplanでも明示横位置を照合し、同じ幅のまま1 rawずらした指定は従来modeと新modeの両方で拒否する。

反復header／captionで同じ元unitに異なる枠が必要な場合はtable_repeated_frame_reflowを返す。既存の一つのprofileで片方の区間を上書きしない。表内blockの親枠が変わる場合もtable_block_frame_reflowとして残し、未接続のblock横位置を使った出力を許可しない。枠が不変のblockは再計測した親枠との照合を受ける。区間ごとの全階層projectionと課金済み線形owner探索を使う現実装は、元全巻の性能受入を済ませたものではない。

合成フォントと無変更原ノ味の本文／脚注で、通常継続・段落途中の分割・caption内の強制改ページの12ケースを実driver PDFへ接続した。通常／強制は3ページ、分割は5ページとなり、原ノ味の脚注分割では幅循環を検出し、1回の行末保持refinementと11回の幅feedbackで安定した。元font字幅・宣言master・2列の比率とPDF ParentTreeの段落対応を用いた独立検査は24 PDF／88ページに成功し、位置・文字・欠落・重複・ページ数の144改変を拒否する。PDFの構造IDは元node IDそのものではないため、生成table sectionやnote linkを含む元構造から対応を独立に再構成する。

統合195 tests、独立595 PDF（実driver 288件）／1,942ページ／4,533改変拒否、既存571 PDFとVMB table 2 PDFのbyte一致を確認した。詳細な予算境界、回帰・描画確認と証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-source-profiles-design-14210)を参照する。

§14.207〜14.209で未接続としていた異幅表段落のdriver PDF処理は本節で接続した。異なる枠を持つ反復header／caption、表内blockの可変枠、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・著者および人手受入は引き続き未完であり、設計全体の完了とは扱わない。


### 14.211. 異幅の表内blockへ親枠の幅・横位置を再binding（2026-09-10）

[ADR-0077](../adr/ADR-0077-book-2-table-block-source-profiles.md)に従い、元flowのblock幅指定に、同じ元block順の本文基準横位置を組み合わせる候補を追加した。全block幅と同じ個数を要求し、元owner順の不一致、別flow、幅指定の欠落、不正な長さ、元本文／脚注本文envelope外の指定を拒否する。明示横位置と表内block幅継承modeの併用は拒否し、frameless組版でも破棄しない。

各実shaping passで元block幅を適用した後、新しいframeの既存region recordへ横位置を保持する。元の最大親枠と継承親枠は別に残し、脚注の許可範囲から元の生成markerを除く。幅は元親幅を上限とし、元ownerと横位置を幅のfingerprintに続く専用domainへ含める。source event訪問とregion照会を共通workへ課金し、layoutで横位置の別copyを確保しない。

区間別feedbackでは表外blockを従来の物理幅で観測し、表内blockは継続区間の親幅で元表階層を再計測する。元順序の幅・横位置候補と訪問vectorを確保前に課金し、元block ownerに対応する幅と横位置を上書きする。観測したblockは意味的に一回消費する。反復する同じblockの幅または横位置が一致しない場合はtable_repeated_frame_reflowを返し、一つのsource候補で異なる描画を代用しない。

driverはblock横位置候補がある場合に明示幅と組み合わせ、候補がない既存経路では継承modeを保つ。全横位置を候補比較・循環digestへ含め、比較訪問も課金する。段落とblockを同じ新しいsource flowへ再bindingし、既存の生成番号変更時の候補破棄、累積work／record／pass上限と最終PDF候補の連続一致を維持する。

図版・PNG・JPEG・SVG・native数式・precomposed vectorの既存geometry処理が新しい親枠を使う。固定寸法、元resource・source identity、揃え、captionと数式番号のminimum gapを保持し、固定objectを縮小して列へ収めない。最終math処理では実区間を独立に再計測し、block親枠の幅と横位置を照合する。異幅がないplanでも明示block横位置を元階層と照合し、幅を保って横位置だけ1 rawずらした指定を従来modeと区間別modeの両方で拒否する。

元の1:2比率列、caption内の強制改ページとcell内の元block列を使い、合成フォントのvector・native・PNG・JPEG・SVGの本文／脚注10ケース、無変更原ノ味の図版4形式の本文／脚注8ケースを実PDFへ接続した。本文は4ページ、脚注は2ページとなり、全ケースで2回の幅feedback、4回のline pass、4回のpage passで収束する。専用独立検査は元master・比率列・字下げ・font metricsからblock位置、固定寸法と数式番号を照合し、36 PDF／108ページで成功、108改変を拒否した。

統合198 tests、独立631 PDF（実driver 306件）／2,050ページ／4,875改変拒否が成功し、既存595 PDFとVMB table 2 PDFはbyte一致する。元source幅／横位置の不正指定、累積work／record境界と回帰証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-block-source-profiles-design-14211)を参照する。最初のnative試験ではテスト用tableのsource spanが内包する複数の元blockを覆っていなかったため、tableの範囲だけを修正した。元数式やspan検証を弱めていない。

§14.210で残したtable_block_frame_reflowは本処理へ置き換えた。反復する同じ段落・blockに異なる物理枠が必要な場合、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入は未完である。課金済み線形owner探索と区間ごとの全階層再投影を使う現実装を、元全巻の性能受入とは扱わない。


### 14.212. 収束した行末を保持して独立した組版variantを再構築（2026-09-10）

[ADR-0078](../adr/ADR-0078-book-2-line-variant-seeds.md)に従い、反復headerの幅ごとに異なる行末・字形分割・高さを持つための再構築元を追加した。実際のshape／line収束後にだけseedを作り、元段落のshaping context行末と観測済みの安定fingerprintを保持する。行末は既存のUTF-8 context表現に従い、atomic placeholderと強制改行を含む。現在の行番号を別の組版へ流用せず、元source flowを書き換えない。

seedは元flow、policy、admitted resource、vector／native binding、limits、本文／page plan、幅・横位置指定を不変の借用として保持する。再構築APIはseedと予算だけを受け取り、別の生成番号・resource・幅指定を途中で差し替えられない。選択行末のcopyとseed recordはcallerの先行recordと観測graph chargeを基準に確保前に検査し、capture走査を収束処理と同じcandidate／frame workへ課金する。最初のshaping passは既存のstageごとのrecord上限を使い、本部品を複数variant全体の完全なallocation accountingとは扱わない。

再構築時は元の観測済みgraph上限と新しいinput view・結果recordを、seed／callerの先行chargeへ加えてshaping前に検査する。不変の同一入力と保持contextから、実shaping・inline準備・frame／line選択・footnote bindingを再実行する。観測した安定fingerprintとの一致と、graph chargeが事前計上した上限内であることを確認してから、所有callback内だけへ結果を公開する。再構築candidate／frame workとinput・比較訪問を課金する。

二つのvariantを同時に保持しても、shape・prepared inline・line・footnoteの所有者は独立し、元flowと不変のnative計算結果は共有する。別variantのprepared ownerやfootnote receiptを混用すると拒否する。元sourceの一致だけで異なる物理配置を許可せず、段落の行数と実table計測のheader高さをそれぞれの組版から求める。

合成フォントと無変更原ノ味の本文／脚注headerで、本文root親幅140／220pt、脚注root親幅140／200ptのseedを作り、独立した二つの実組版とtable計測を同時に保持した。脚注のroot親幅はmarker・gapを除いた上限内とする。10段落の行数が変わり、合成フォントのheaderは92／46pt、原ノ味は92／69ptとなる。別のnative試験では異なる行組みが元の同じ計算ownerを共有し、それぞれのgraphで元inline atomを一回保持する。capture／再構築のwork・record exact成功と1不足拒否、別flow指定と所有者混用の拒否を確認した。

統合200 tests、独立631 PDF（実driver 306件）／2,050ページ／4,875改変拒否が成功し、既存631 PDFとVMB table 2 PDFはbyte一致する。部品のcandidate／frame workと統合証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-line-variant-seeds-design-14212)を参照する。

実ページごとのvariant選択、反復headerの高さ予約と配置、反復source closure・PDF assemblyへの接続は未完であり、異なる反復枠のtable_repeated_frame_reflow拒否は維持する。本節は新しい反復異幅PDFの受入ではない。残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入も未完である。


### 14.213. 複数の実際の行組みを反復処理で同時保持（2026-09-10）

[ADR-0079](../adr/ADR-0079-book-2-line-variant-sets.md)に従い、収束済みseedの非空・順序付き集合から、入力context、実shape、prepared inline、選択行、footnote viewの各層を別々に一度確保する。各層を保持した一つのcallbackへ全variantを渡し、件数に比例するcallbackの再帰を使わない。同じseedを複数指定しても独立したshape／line ownerを作る。

確保前に元flow・policy・resource・vector binding・native計算の同一owner、limitsと日本語改行modeの一致を要求する。内容fingerprintが同じでも別flow／native ownerの混用を拒否する。各seedのbody・page・幅／横位置指定は不変のまま再構築し、全variantの安定fingerprintと観測graph上限を確認した後だけcallbackを呼ぶ。順序と各seedのfingerprintを集合digestへ含める。

seedの保持contextと先行chargeを分離し、最大先行chargeへ全指定contextを加え、callerの大きいledger、全再構築graph上限、入力view、5層のownerと結果recordを確保前に課金する。重なる先行chargeや重複seedは保守的に多く数える場合がある。全viewは集合のrecord chargeを返し、互換性検査・各層走査・実candidate／frame処理・最終照合を共通workへ含める。最初のseed captureや後段body／table／page／PDFを含む全pipelineのallocation受入とは区別する。

合成フォントと無変更原ノ味の本文／脚注で狭幅・広幅・狭幅の3件を同時保持し、別ownerのtable計測とheader高さ差を照合した。32件の反復再構築、順序digest、別flow／native owner、空集合、work／record exact成功と1不足拒否も確認した。統合200 testsと独立631 PDF検査が成功し、既存631 PDFおよびVMB表2 PDFはbyte一致する。詳細は[進捗記録](28-vmb-book-production-progress.md#book-2-line-variant-sets-design-14213)を参照する。

実ページのvariant選択、反復headerの高さ予約・配置、反復source closureとPDF接続は残り、table_repeated_frame_reflowを維持する。残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入も未完である。


### 14.214. 別の実際の行組みに結び付いた反復header geometry（2026-09-10）

[ADR-0080](../adr/ADR-0080-book-2-table-header-variants.md)に従い、意味sourceを持つ元table計測と別variantのtable計測を結び付けるheader候補を追加した。両方の選択行が同じ再構築集合の正確なownerであること、limits・元table owner・親table・脚注定義の対応を要求する。fingerprintが同じでも集合外の別計測は拒否する。結果は両計測を借用し、検証時も同じ組を要求する。

variantの実際の行数とrow bandから、root captionを除いたheader高さを求める。反復処理でheader cellをsource順に訪問し、内部の入れ子表はcaption・header・bodyを全て含める。入れ子captionはcell ownerなし、元強制改ページは非描画のまま保持する。各描画leafはvariant側だけのglobal item位置・相対top・元cell ownerと、元段落logical unit範囲またはblock ownerを持つ。元計測のleaf番号へ読み替えず、意味sourceの二重消費にも使わない。

容量から実variant高さを引き、1 raw不足ならTableHeaderOversizeを返す。これは部品の容量計算であり、既存page searchの予約を変更したとは扱わない。全計測graphのtask／leaf上限を確保前に数え、再構築集合と両計測のcharge、大きいcaller ledger、保持vectorと結果を保守的に課金する。入力の先行chargeが重なる場合も過少計上しない。各vectorは一度確保し、owner照合・走査・固定長hash計算を共通workへ含める。

合成フォントと無変更原ノ味の本文／脚注で、通常headerと入れ子表・caption・強制改ページを持つheaderを検証した。期待する段落ownerは元JSONのroot headerから取り出し、全元unitの一回coverage、root caption／bodyの除外、幅による実高さ・leaf数変更を照合する。別owner、集合外の同一fingerprint、不正table、work／record／容量のexact成功と1不足拒否を含む結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-variants-design-14214)を参照する。

候補はページ選択・source closure・PDFの許可ではない。物理継続区間でのvariant選択、page searchの高さ予約、異なる行組みの反復leafを保持する配置・source closure・PDFへの接続は残り、table_repeated_frame_reflowを維持する。今回の専用fixtureは段落geometryを検証し、新しいblock header PDFや元全巻の受入は主張しない。残る名前指定、柱・footer content、段組、公開経路・manifest・管理ホスト・性能・著者および人手受入も未完である。


### 14.215. 実header高さを使う表の継続区間選択（2026-09-10）

[ADR-0081](../adr/ADR-0081-book-2-table-header-selection.md)に従い、§14.214のheader候補を表の継続区間選択へ接続した。元table計測・table位置・limits・cursorを照合し、元header／captionを消費済みの継続に限って受理する。初期・終端・別計測のcursorやheaderの混用を拒否する。

元source座標、row bandとcaption境界には元header高さを使い、一回の継続選択の容量予約・残量・本文cellのtop・retry・使用高だけに指定variantの実高さを使う。共通cut、独立cell、rowspan、入れ子tableへ接続し、成功・no-fit・errorの後には通常modeへ戻す。失敗試行の累積work／保持候補のchargeは残す。

元root headerの旧leafだけを抑制し、本文cell内の子tableが独自に反復するheaderは元計測のまま保持する。選択結果は元source消費を持つfragmentと指定headerを結び付け、描画leafごとに正しい計測owner・global item位置・top・反復属性を返す。元header variantのleaf番号を本文の行番号へ読み替えない。元cursorとsemantic範囲の一回消費を保持し、選択fingerprintへ両geometryの対応を含める。

headerの共有set／元計測chargeと、独立したvariant計測・projection上限を分離した。大きいcaller ledgerが独立計測をmaxの内側へ隠さないようにし、searchは遭遇したvariant計測を正確な参照で保持・一度ずつ課金する。header projection上限は毎試行で保守的に前払いし、同一headerの再使用と別ownerの併存を過少計上しない。registryの保持・照会・exact拡張時のcopy上限と結果recordも確保前に課金する。

合成フォントと無変更原ノ味で、本文／脚注、共通cut・強制改ページ・rowspan・入れ子header・本文内の入れ子table、狭幅／広幅の元計測を組み合わせた40ケースを検証した。同じcursorからheaderを変えると本文容量が変わり、大小両方向の切り替えでも全元itemを一回消費する。各leafのownerと高さ、子headerの反復保持、no-fit後の通常選択復元、累積work／record exact成功と1不足拒否を確認した。詳細は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-selection-design-14215)を参照する。

結果は別の型とし、内部の単一owner用fragmentを公開しない。既存の混在本文／脚注ページ選択・source closure・PDF経路は、複数の計測ownerと実glyph／resourceを扱う接続が必要である。本節は実table fragmentの選択であり、物理masterごとの自動variant選択や新しい反復異幅PDFの成功ではない。table_repeated_frame_reflow拒否を維持し、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入も未完である。

### 14.216. 物理ページ枠からの反復header候補選択（2026-09-10）

[ADR-0082](../adr/ADR-0082-book-2-table-header-catalog.md)に従い、元root table・実parent幅・正確なheader計測参照を持つcatalogを追加した。空・重複・逆順・別baseを拒否し、幅は候補の実frameから取得する。元の表階層をその幅で独立に再計測し、header各行の幅とsource-unit開始位置、blockの存在するframeを照合する。root幅が一致していてもleafの実幅が1 layout unit違う候補を受理しない。

本文・脚注の候補列挙と選択へcatalogを接続した。物理ページの本文／脚注幅から、元の階層のinset・脚注marker/gapを保持したroot parent幅を求め、元headerを消費済みの継続区間に対応候補を選ぶ。実header高さで容量・cell位置・retry境界を計算し、元source範囲を一度だけ消費する。初期headerは元計測を使い、対応幅がない継続はtable_header_variant_widthで拒否する。

catalogはsource/demand stateの発行前にだけ設定できる。元flowとlimitsを照合し、共有入力の履歴とは別に独立計測・projectionを前払いする。探索へ設定する際は既存探索ledgerへcatalog全chargeを保守的に加算し、片方のgraphをmaxの内側へ隠さない。候補の保持、階層再計測、二分探索と失敗試行を累積work／recordへ含める。

§14.215の選択型から混在ページ選択への変換はscheduler内部に限定する。変換後もheaderの正確な参照と合成fingerprintを保持し、variant描画leafごとに実計測ownerを返す。既存の単一owner用cell／caption／source配置queryはtable_header_variant_placementを返し、別の行組みのitem位置を誤って配置しない。

合成フォントと無変更原ノ味、本文／脚注、通常／入れ子headerの8ケースで、交互に変わる140pt／220ptの物理ページ幅から両候補を選択する。元sourceの一回消費、反復leafの計測owner、候補順序・幅の不整合拒否、catalogとページ探索のwork／record境界を検証する。検証結果とコマンドは[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-catalog-design-14216)に記録する。

今回の入力は明示的に再構築した候補setである。private PDF driver内での候補生成、複数ownerを保持する配置・source closure・実glyph／resourceの閉包・PDF組立は残る。配置を使う安定geometry検証も未接続であり、table_repeated_frame_reflow拒否を維持する。新しい異幅反復header PDFや元全巻・公開経路の完了は主張しない。残る名前指定、柱・footer content、段組、manifest・管理ホスト・性能・著者および人手受入も未完である。

### 14.217. 実header計測ownerを保持する混在ページ配置（2026-09-10）

[ADR-0083](../adr/ADR-0083-book-2-table-header-placement.md)に従い、§14.216で選んだheaderを本文／脚注の実配置へ接続した。配置ページは、反復root headerの各断片について正確なheader参照とvariant側global item位置を保持する。元本文・脚注と本文cell内の子header反復はbase計測を使い、variantの位置は対応する実body／definition streamへ変換して解決する。

断片のbounds・baseline・block viewportを実計測から作り、元cell roleを持たない入れ子captionの反復属性も保持する。リスト記号・脚注記号は共通の断片単位処理へ分け、variantページでは各断片の実flowからbindingを照会する。リスト記号は反復して描画し、脚注定義markerは反復headerから再発行しない。文字・viewport・記号をそれぞれの計測原点から物理ページの本文／脚注原点へ移動し、数式番号も実flowのblock／shapeから配置する。

保持する対応情報の件数と拡張時のcopy上限を確保前に課金し、照会・配置・安定化試行を既存の累積work／recordへ含める。安定geometry比較へheaderの正確な参照とitem位置を加え、実配置を伴う反復比較を可能にする。variantがないページは従来の処理と予算を維持する。

合成フォントと無変更原ノ味、本文／脚注、リストを含む通常header／入れ子headerの8ケースで、140pt／220ptの交互幅と独立した本文／脚注の横位置を検証する。全配置断片を実計測から解決し、source・item・幅・高さ・baseline・header内top・記号位置・caption反復を照合する。元sourceの一回消費と累積work／record境界も確認し、結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-placement-design-14217)へ記録する。

単一owner用table leaf queryは引き続き拒否し、混在配置はowner付きqueryを使う。variantを持つページのsource closureはtable_header_variant_source_closureで止め、元unitの反復source照合と実glyph／resource閉包ができるまでPDF権限を発行しない。今回の専用fixtureは段落・リストの配置を検証し、新しいblock headerや番号付きblock headerのPDF受入は主張しない。private driver内のcatalog生成とPDF接続、table_repeated_frame_reflow、残る名前指定、柱・footer content、段組、元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入は未完である。

### 14.218. 反復headerを元logical unitへ照合するsource closure（2026-09-10）

[ADR-0084](../adr/ADR-0084-book-2-table-header-source-closure.md)に従い、実配置した別行組みの反復headerを元sourceへ照合する処理を追加した。元段落indexとowner、元行のlogical unit範囲を保持する索引を作り、別variantの行番号を元の行番号へ読み替えない。一つの反復行が元の複数行にまたがる場合と、元の一行が複数の反復行へ分かれる場合を扱う。空範囲は元の空行を要求し、範囲外・欠落・重複を拒否する。

選択したheader leafと配置側の対応を順序どおり照合し、正確なheader参照、variant側global item位置、元owner、本文／脚注local item位置、cell／captionの反復属性、header内topを要求する。余分・欠落・逆順の対応は受理しない。blockは元block sourceとの一致を要求し、強制改ページを描画leafにしない。反復は元sourceの反復visitへ記録し、本文・参照済み脚注の意味sourceは引き続き一回だけ消費する。

各断片の実flowからsource・geometry・baseline・vector／native math receiptを検証し、意味断片と反復断片を別々に数える。検証結果はvariant断片数と範囲検査付きの実flow解決APIを持つ。索引の確保・整列・検索・区間走査・対応照合を既存の累積work／recordへ含め、variantなしの従来経路は予算と出力を維持する。

合成フォントと無変更原ノ味、本文／脚注、リストを含む通常header／入れ子header、狭幅／広幅の元計測を組み合わせた16ケースで、140pt／220ptの継続幅、安定配置、元sourceの一回消費と実flow解決を検証した。区間境界と累積予算のexact成功・1不足拒否を含む詳細は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-source-closure-design-14218)を参照する。

§14.217のtable_header_variant_source_closure拒否を実source照合で置き換えた。単一計測を前提とする後段の幅feedbackとmath terminal生成には、それぞれtable_header_variant_width_feedback、table_header_variant_math_terminalsを設ける。実glyph／resource閉包、後段consumerの実owner対応、private driver内のcatalog生成とPDF接続は残り、table_repeated_frame_reflowを維持する。今回の専用fixtureは段落・リストを扱い、新しいnative／vector／block header PDFや元全巻・公開経路の完了を主張しない。残る名前指定、柱・footer content、段組、manifest・管理ホスト・性能・著者および人手受入も未完である。

### 14.219. 反復headerの実frameを元sourceの幅割り当てと分離（2026-09-10）

[ADR-0085](../adr/ADR-0085-book-2-table-header-width-feedback.md)に従い、source closure済みのvariantページから表の幅観測と段落frame feedbackを作る処理を接続した。各観測pieceは元logical unit範囲・反復属性に加え、別計測のheaderであることを明示する。再計測付き報告はその実開始位置・幅も保持する。variantのglobal itemを実計測の本文／脚注local位置へ変換し、実選択行・blockから解決する。本文cell内の子header反復は元計測のまま扱う。

元表階層を物理root parent幅で再計測し、別headerの実幅・開始位置と照合する。元headerの意味sourceは初回配置の幅を持ち、異幅の反復観測はその割り当てを上書きしない。段落／blockのfeedbackでは、variant位置を元flowへ照会する前に識別し、表の観測報告から実frameを検証する。元本文の幅不一致・全unitの一回coverage・従来の子header反復の整合性判定は保持する。

source profileの生成と最終table frame検証で、反復属性・独立再計測frame・実variant frameの一致を要求する。最終段落幅検証も先に表のsource frameを検証してからvariantの元flow照会を省く。循環時に保持する改行境界は元意味行から取得する。件数を先に数えて一回確保し、検索・再計測・走査・hashを累積work／recordへ含める。報告fingerprintにはvariant区別と実frameを加え、variantなしの従来encoding・予算・PDFは維持する。

合成フォントと無変更原ノ味、本文／脚注、リストを含む通常／入れ子header、狭幅／広幅の元計測で、raw／再計測報告、実配置と元unit範囲、意味sourceへの幅割り当て、異幅反復による非上書き、保持改行境界と予算境界を検証する。詳細な結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-width-feedback-design-14219)へ記録する。

単一幅の段落／root table／block APIはvariant closureに対するtable_header_variant_width_feedbackを維持し、複数観測にはparagraph_frame_feedbackとtable_width_framesを使う。今回のfixtureは元本文の幅が未収束であり、feedbackのmatches=falseを正しく返す。これは候補生成の接続であり、最終幅の受入ではない。table_header_variant_math_terminals、実glyph／resource閉包、private driver内のcatalog生成・PDF接続とtable_repeated_frame_reflowは残る。専用native／vector／block header PDF、残る名前指定・柱・footer content・段組・元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入も未完である。

### 14.220. 実header計測からの数式terminal生成（2026-09-10）

[ADR-0086](../adr/ADR-0086-book-2-table-header-math-terminals.md)に従い、別行組みのheaderを持つsource closureを数式terminal生成へ接続した。table_header_variant_math_terminalsの一律拒否を置き換え、元表階層の独立再計測と段落／blockの物理幅検証を要求する。元本文幅が未収束ならWidthMismatchで拒否する。

各配置断片の実flowを解決してから、実選択行のinline数式・vector block・native displayを取得する。元receiptの正確なowner、物理原点・baseline・viewport、cell role・反復属性を保持し、意味数式と反復数式の件数をsource closureへ照合する。native計算とvector bindingを元のまま共有し、反復描画によって元数式を二重消費しない。式番号は実配置のgeometryを保持する。

variant付きcanonical bytesには、従来の数式・式番号recordの後へowner対応を追加する。40 byteのtag／件数headerと、各断片180 byteの物理page・local fragment・variant global item、header／選択行／block／vector binding／native計算のfingerprintを含める。数式を持たないheader断片も対象とし、variantなしのbytesは維持する。全拡張長を確保前に検査してspoolへ加算し、照会・走査・全canonical hashを累積workへ含める。

ネイティブ／ベクトル数式、本文／脚注、元行の分割／結合方向の8ケースを検証する。合法な保持改行境界により、同じ有効な物理幅でも元headerと反復headerの行番号・高さが異なる計測を作る。実inline／block receipt、元数式の一回消費、反復回数、vector図、式番号、canonical owner対応とwork／record／spoolのexact成功・1不足拒否を照合する。既存の16異幅ケースでは未収束の元本文を引き続き拒否する。結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-math-terminals-design-14220)へ記録する。

text・marker・figure・font/resourceの後段が実flowを扱うまでは、display builderをPendingHeaderVariantsで止める。専用fixtureは合成フォントと一定の物理幅を使う数式terminal／geometry検証であり、新しいvariant PDFや原ノ味数式header、異幅PDF収束の受入ではない。実glyph／resource閉包、private driverのcatalog生成・PDF接続とtable_repeated_frame_reflow、残る名前指定・柱・footer content・段組・元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入は未完である。

### 14.221. 実header flowからの描画とfont使用列挙（2026-09-10）

[ADR-0087](../adr/ADR-0087-book-2-table-header-display.md)に従い、variant付き数式terminalをbody displayへ接続した。terminalに物理断片の総数と、variantがある場合だけ各断片の正確なflow参照を保持する。件数とvectorを確保前に課金し、既存の検証済み走査で埋める。範囲検査付きの定数時間resolverを公開し、glyphごとの前ページ再走査を避ける。variantなしは追加配列を持たず従来予算・bytesを維持する。

display builderは各headerの実flow・shape owner・admitted resource・limits・vector bindingを照合し、走査を累積workへ含める。文字のpreflightと描画を実選択行から行い、正確なcluster／fontと物理glyph座標を保持する。画像／vector、配置anchor、非描画行も各実flowから解決する。未配置anchorは元意味sourceの記録として保持し、反復から別のsource義務を作らない。

リスト／脚注記号はpage-local fragment位置を検査付きprefix件数でglobal位置へ変換し、preflightと描画の両方で実shapeを取得する。式番号は既存のglobal位置から実block／number shapeを解決する。font使用列挙も各paintの実shape／native font-instance tableを用い、元字形・text／scalarとadmitted fontの同一性を維持する。

ネイティブ／ベクトル、本文／脚注、元行の分割／結合の8ケースへ反復リスト記号とanchorを追加した。全text drawの実cluster／font、元glyph IDと物理座標、式番号・記号・vector図・anchorの実owner、font-instance table fingerprint、global resolverの件数・範囲、累積work／recordのexact成功・1不足拒否を照合する。結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-display-design-14221)へ記録する。

PendingHeaderVariantsはdisplay builderからbody displayのresource検証へ移し、具体的な描画とfont使用一覧を検査できるようにする。glyph／font／resource閉包とPDF接続は未受入であり、後段のresource-selection／PDF権限は引き続き発行しない。専用fixtureは一定物理幅で保持改行境界を変える合成フォントの検証で、新しいvariant PDF、原ノ味variant描画、raster／非描画headerの専用受入ではない。private driverのcatalog生成・幅収束とtable_repeated_frame_reflow、残る名前指定・柱・footer content・段組・元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入も未完である。

### 14.222. 実反復headerの字形・画像リソース閉包（2026-09-10）

[ADR-0088](../adr/ADR-0088-book-2-table-header-resources.md)に従い、実variant描画をglyph／image選択、font closure、subset生成、CID計画へ接続した。resource選択用の同一性検証をPDF受入用から分け、実displayの正確なadmitted ledgerとlimitsを要求する。各headerの実flow・shape・bindingはdisplay構築時に同じledger／limitsへ照合済みである。PDF consumer用のverify_resourcesはPendingHeaderVariantsを維持する。

実paintのfont-use一覧から元glyph IDを集め、元headerと反復headerの和集合を選択する。shape／nativeのfont-instance表は同じ全admitted face一覧から作られるため、各使用のtable fingerprint一致を引き続き要求し、異なる番号体系をnumeric IDだけで統合しない。font容量上限、確保・整列・hashと累積work／record／spoolを維持する。closure・subset・CIDは各使用の元source・text／scalar・paint／slotを保持し、画像はpayloadを共有して物理使用を省かない。

従来のnative／vector × 本文／脚注 × 分割／結合の8ケースに、無変更原ノ味CFFの4ケースと、文脈GSUBの4ケースを加えた。専用の再生成可能なテストフォントでは、f＋space＋iの文脈だけf（GID 71）をZ（GID 59）へ置換し、f＋space後の合法な保持改行で元字形へ戻す。分割方向ではGID 71が全元text描画に存在せず、反復headerだけに現れることを要求し、その字形が選択・subset・CIDへ含まれることを確認する。原ノ味の専用ケースは同じLatin文字fixtureでCFF処理を検証し、日本語header全般の受入とは扱わない。

実使用からの独立したglyph集合、正確なfont owner、subset対応・advance・CID抽出、画像共有と全物理使用、累積予算のexact成功・1不足拒否を照合する。テスト用probeの元承認済みfont／生成program／mapping／CIDをFontToolsで別途読み、全輪郭・横metrics・CFF cmap・CID幅・Unicodeの曖昧性と抽出を検証する。結果と統合回帰は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-resources-design-14222)を参照する。

これはリソース閉包の接続であり、variant PDFの受入ではない。private driver内のcatalog生成・幅収束とtable_repeated_frame_reflow、PDF consumer接続、raster header、残る名前指定・柱・footer content・段組・元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入は未完である。

### 14.223. 実header variantのPDF構造・描画・埋込み（2026-09-10）

[ADR-0089](../adr/ADR-0089-book-2-table-header-pdf.md)に従い、実display ownerとresource closureが検証済みのvariantをPDF pipelineへ接続した。PendingHeaderVariantsを削除し、resource選択とPDF entryの両方で正確なadmitted ledger／limitsを引き続き要求する。各実headerはdisplay構築時に照合済みであり、glyph選択も各使用のfont-instance表を検証してからclosure／encodingへ進む。

PDF consumerを確認し、structure・table／note relation・navigation metadata・page masterは共有する元意味sourceを参照し、描画・物理geometry・anchor・fontは実displayから取得する。variantの行番号で元計測を再照会しない。独立検証用の非描画行probeもglobal fragmentの実flowを使う。元anchorがdestinationを所有し、反復headerはArtifactとして元sourceの二重消費・destinationの置換を行わない。

専用の実flow照合に加え、既存のbody／resource／structure／navigation／assembly検証をvariantへ適用した。stateful pipelineのbyte一致、累積work／record／spool／outputのexact成功・1不足拒否、prior workの伝播、失敗後の非返却を確認する。従来のnative／vector・原ノ味CFF・文脈GSUBの16ケースへ、本文／脚注のraster画像とanchorだけの非描画headerを分割／結合両方向で加え、20 PDFを扱う。rasterケースは固定画像とheaderが収まる160pt領域を使い、画像を縮めて成立させない。他のケースは100pt領域と有効な一定240pt幅を維持する。

既存の独立PDF検証で元structure・marked content・Artifact・navigation・参照・object graphを照合する。さらに実display fingerprintでresource probeとPDFを結び、埋込みprogram hash、CID幅／map、ToUnicode、ページ内全CID描画の多重集合を比較する。反復だけに現れる文脈字形はArtifact内だけで描画されることを要求する。program・CIDを改変して再serializeしたPDFを拒否し、文脈ケースではArtifact属性の除去も拒否する。原ノ味本文／脚注の先頭・反復ページをPopplerで描画して、改行差、vector／式番号の位置、脚注定義markerの非反復を確認する。結果は[進捗記録](28-vmb-book-production-progress.md#book-2-table-header-pdf-design-14223)を参照する。

これは専用に構築したcatalogからのvariant PDF受入である。private driver内の自動catalog生成・異幅収束とtable_repeated_frame_reflow、残る名前指定・柱・footer content・段組・元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入は未完である。専用Latin fixtureの原ノ味検証を、日本語全巻やPDF/UA・人手受入、VMB producer／Go testの完了へ拡大しない。


### 14.224. private driverの反復header幅検出・catalog自動生成（2026-09-11）

[ADR-0090](../adr/ADR-0090-book-2-automatic-table-header-catalog.md)に従い、空のsealed catalogから実際の本文／脚注ページ探索を開始し、不足する継続幅を元table index・source owner・parent幅を持つTableHeaderWidthRequiredとして取得する。元階層・list inset・脚注marker/gapを保持し、未検証headerへのfallbackは行わない。幅要求は順序と重複を検証して蓄積し、失敗した探索のwork／recordと各探索のpage passを返却しない。

元の収束seedと同じsource・policy・admitted resource・vector binding・native計算・page planから、別のsource幅割当てを持つsiblingを再収束する。元rootのmeasurement幅をsource event順で収集し、要求されたrootだけを指定幅へ変える。反復headerには元本文のsource-unit幅／原点や保持行末を引き継がない。各variantのline・number・block・table・headerの所有層を同時に保持してcatalogをsealし、全vector容量・走査・再構築・行pass・投影を制限内で計上する。

private PDF driverは、通常の幅feedbackがtable_repeated_frame_reflowを返した場合に、消費済み予算を保持して自動catalog経路へ入る。実variantのsource閉包にはparagraph_frame_feedbackを使い、本文幅とページ参照labelの既存収束ループを継続する。前のsource/labelのcatalogを再利用せず、stable pages、最終物理幅、source閉包とPDF pipelineを改めて要求する。通常のheader不要経路の計上は維持する。

再試行ごとの履歴の重複計上を除くため、table measurementの過去記録と新たに保持するblock/body/table投影を分けた。variant setの正確な所属照合を前提に、共有履歴の最大値と独立投影を保持する。候補選択は新しいvariantの保持分を計上する。sealed catalogから直接searchを構築する入口はcatalogの全記録をsearch確保前に支払い、bind時の二重加算を避ける。従来の独立setterは保守的な全catalog計上とidentity/state検証を維持する。

本文／脚注・通常／入れ子header・無変更の原ノ味日本語sourceで、140ptと220ptの幅に収束した実private-driver PDFを検証する。空catalog、foreign owner、幅順序、work／record／行pass境界と1不足拒否、実native/vector計算の共有も確認する。証跡とローカル回帰結果は[進捗記録](28-vmb-book-production-progress.md#book-2-automatic-table-header-catalog-design-14224)を参照する。

今回のfixtureは累積line/page passを128まで明示的に許可する。各本文幅passでcatalogを再生成する費用は計上し、全巻性能の受入へ拡大しない。異幅native/vector/rasterの実driver追加検証、反復幅に現れない固定body objectと内側body table headerの到達性、依存crateだけでstagingを有効にしたworkspace feature構成、残る名前指定・柱・footer content・段組・元全巻・公開経路・manifest・管理ホスト・性能・著者および人手受入は未完である。

### 14.225. 依存crateのfeature統合と契約境界（2026-09-11）

[ADR-0091](../adr/ADR-0091-book-2-feature-unification.md)に従い、依存crateだけでbook-v2-stagingが有効になる構成でも、共有SemanticBlock／WireSemanticBlockのDescriptionListとpayload型、CffV2 shaping errorのenum形状を一定にした。共通consumerのmatchは明示的な処理・拒否を常に持ち、消費側の独立したfeatureで分岐を消さない。Book2のvocabulary・入口・実lowering／layout処理は引き続きfeatureで隔離する。

carrier型の存在を契約の受入と同一視しない。sealedなWireSemanticKindと既存DESCRIPTION_LISTS検証により、凍結契約への空／非空description_listのdecode、手作業で作った型からのencodeを拒否する。外部integration testはlibrary側のcfg(test)に頼らず、通常構成とstaging構成でこの境界を確認する。

14 crateそれぞれのstaging featureを単独で有効にしたworkspace checkと、通常featureの9 crateの境界テストを検証した。§14.224に記録した依存feature構成のコンパイル不備を解消する。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-feature-unification-design-14225)を参照する。公開profileや契約1.5のCLI受入を追加する変更ではない。

### 14.226. 異幅の数式・画像headerを実driverからPDFへ接続確認（2026-09-11）

§14.224の実private driverに対して、本文／脚注のnative数式、vector数式と式番号、raster画像とanchorだけの非描画行、および無変更原ノ味CFFの8ケースを追加した。各PDFで3種類の物理幅と異なる原点、実header variant、元数式の一回消費・各継続ページでの反復、式番号数と実terminal kindを照合する。画像と非描画行も各ページに一つ、元意味sourceとしては一つだけ存在することを要求する。

nativeの本文／脚注とvectorの本文は100・140・180pt幅を使う。vector脚注の成功ケースは120・160・200pt幅を使い、元vectorの固定metricsとlist／脚注insetを維持する。100ptのvector脚注ではNoFeasibleLineを返してPDF callbackへ到達しないことを別テストで確認する。ページ高は既存fixtureの100pt、rasterケースでは160ptを維持する。

実displayからglyph集合・subset・CIDを検証してprobeを保存し、独立PDF検証へ接続する。既存のbody/resource検証ヘルパーは入力からの累積workに追加の検証枠を加え、固定総量だけで既に消費したworkを拒否しない。exact成功・1不足拒否、失敗後の非返却、prior伝播の検証は保持する。結果と既存PDFのbyte比較は[進捗記録](28-vmb-book-production-progress.md#book-2-header-math-driver-design-14226)を参照する。

これは専用fixtureの追加受入であり、固定non-header body objectの到達性、内側body table headerの独立反復、残る名前指定・柱・footer content・段組・元全巻・公開CLI・manifest・管理ホスト・性能・著者および人手受入は未完である。原ノ味ケースのLatin sourceを日本語全巻の受入へ拡大しない。


### 14.227. 配置したsourceに限る表の再計測（2026-09-11）

[ADR-0092](../adr/ADR-0092-book-2-table-source-reachability.md)に従い、220ptの先頭ページにだけ置く160ptの図を、140ptの継続header用計測が拒否する問題を修正した。header専用のsource幅割当ては正確なroot tableを指定し、そのheadだけを候補幅で再計測する。rootのcaption／body cellは元envelopeのparentを保持する。head内の子表は全体が反復内容なので、そのcaption／bodyも候補幅を継承する。元の意味source、admitted resource、vector／native計算を削除・置換せず、未使用bodyの計測結果を物理配置の許可へ転用しない。

固定列の子表では、配置後の独立幅検証にも同じ全root走査の問題があった。各物理occurrenceの検証済みsource範囲と実variant leafからownerを収集し、予算内で整列・重複排除する。source eventの走査で必要な祖先を先に確定し、観測したleafに至る表・セル・list／脚注insetとlocal styleを再計測する。未観測の枝は元parentを保持する。対象root自身の固定列は引き続き物理parentへ収まる必要がある。header catalogの独立検査にも同じsource単位の投影を使う。

新しい結果は正確なframe ownerと要求owner列を借用する。段落の照会では元indexとowner、region照会では要求集合への所属を検証する。件数、厳密な順序、root内への所属、leaf種別を確認し、event bitmap／stackの確保、走査、owner収集・整列・検索、hashを累積work／recordへ含める。scopeのrootとowner列をfingerprintへ加え、候補geometryと物理受入を分ける。従来の全root投影APIとその予算／encodingは保持する。

本文／脚注、直接の図／固定列子表内の図、body／captionの8ケースで、図は220ptの先頭ページに一度だけ、headerは140・180・220ptの3幅で反復する。実際の先頭ページが狭い場合や、固定列子表自体を狭幅へ要求した場合は拒否する。範囲外・非leaf・重複・逆順ownerと、work／recordのexact成功・1不足拒否も確認する。証跡は[進捗記録](28-vmb-book-production-progress.md#book-2-table-source-reachability-design-14227)を参照する。

専用の固定native/vector body検証、内側body table headerの独立反復、残る名前指定・柱/footer content・段組・元全巻・公開CLI・manifest・管理ホスト・性能・著者および人手受入は未完である。全sourceの暫定graphを保持する費用や全巻性能の課題を、この到達性修正によって完了扱いにはしない。


### 14.228. 内側の表が独立して続く場合の反復header（2026-09-11）

[ADR-0093](../adr/ADR-0093-book-2-independent-nested-headers.md)に従い、本文セル／caption内の子表が独立して改ページする場合も、実際のページ幅に対応するheader計測を選択する。catalogのkeyは元target table indexと、その祖先rootのparent幅とする。ページ幅の差分を子表へ直接加えず、元の表階層からrootを求め、列・セル・list／脚注insetを再投影する。幅要求errorはtargetのowner／indexとroot parent幅を保持する。

private driverはsource eventからtarget headのleaf集合とrootを求める。source幅割当てに整列したowner集合を保持し、必要な祖先とleafだけを候補幅で再計測する。未選択枝は元envelopeのparentを保持する。target headに含まれる別の子表はcaption／bodyも含めて反復内容である。完全な意味sourceとadmitted resourceを保持し、元本文の再計測結果を別幅への配置許可にしない。rootだけのheaderは従来のscope投影を維持する。

物理regionと正確なcatalog参照を再帰的なchild searchへ渡し、各headerの実高さを確保する。子表のtrial leafに実header参照・header leaf index・相対座標を保持し、rollbackで同時に戻す。選択結果は自身のheaderと子孫のvariantを区別し、いずれかが存在すれば単一ownerの旧配置照会を拒否する。分割の戻し幅、物理配置、幅feedback、source closureは各leafの実計測を参照する。元の意味範囲は一度だけ消費し、反復はartifactとして扱う。後段の数式・描画・font／PDF処理には既存の物理断片ごとの実owner解決を使う。

source収集、投影bitmap／stack、整列・走査・lookup、描画対応とfingerprintを確保前に計上する。新しい対応があるnested fragmentのhashにはheader fingerprintとleaf indexを加える。通常のnested fragmentのencodingは維持する。試行に失敗しても予算を返却しない。

本文／脚注について、親子双方のheader、親headなし、caption内の子表、左右並行の子表、三段の表、無変更原ノ味の日本語を検証する。専用fixtureはline pass上限512／page pass上限128を明示し、意味閉包・物理frame・実header identity・字形／subset／CID・独立PDF検証を行う。予算境界と結果は[進捗記録](28-vmb-book-production-progress.md#book-2-independent-nested-headers-design-14228)を参照する。

16段落の長い初期fixtureでは、128 line passが不足し、line上限を増やした脚注ケースではrecord上限にも達した。受入fixtureの本文／captionと並行／三段の内訳および消費量を記録し、再構築費用や全巻性能を完了扱いにしない。独立child variantの専用数式・画像・rowspan・強制改ページ検証、固定native/vector body、残る名前指定・柱/footer content・段組・元全巻・公開CLI・manifest・管理ホスト・性能・著者および人手受入は未完である。


### 14.229. 同じroot幅のheader計測共有と計上済み投影の保持（2026-09-11）

[ADR-0094](../adr/ADR-0094-book-2-shared-header-replay.md)に従い、同じ元root・同じparent幅で反復する親／子headerを、一つのsibling計測へまとめた。元headのleaf集合を収集・併合し、必要なsource経路の和集合を再投影する。target tableごとのgroup index、意味範囲、header identityとgeometryは分けて保持する。root-only groupは従来のheader scope、複数headerを含むgroupはsource scopeを使う。異なるroot／幅／source／label反復の計測を共用しない。各targetの物理frameはcatalogで従来どおり独立検査する。

正確なcatalogがsearchへ結び付き、search台帳がcatalogの記録数を含み、base measurementが同一で、catalogが正確なheader参照を保持する場合、trialは計上済みのheader投影とmeasurementを借用する。指紋の一致だけでは所有・事前計上と認めない。件数上限を先払いしたidentity走査と新しいselection ownerを計上し、nested paintのコピーは既存leaf allocatorで別に計上する。catalogを持たない公開の単独header選択は従来の保守的計上を維持する。失敗した試行の費用は返却しない。

group対応表、source集合、併合容量、整列・lookupを事前計上し、replay各所有層vectorには従来の保守的容量上限を使う。実際に再構築するsibling数をrequest数から減らすが、productionの記録数／work上限は増やさない。

§14.228でrecord上限に達した16段落の本文／脚注fixtureを、そのまま復元して検証した。本文16ページ・脚注32ページが、明示済みline上限512／page上限128／work上限10億、通常record上限1,000万の範囲で成功する。親子・並行・三段では同じ物理root幅の各headerが正確に同じmeasurementを参照することを要求する。詳細な消費量と回帰結果は[進捗記録](28-vmb-book-production-progress.md#book-2-shared-header-replay-design-14229)を参照する。

大型入力では、検証ヘルパーが既に消費済みのworkより小さい固定総量を使っていた問題も現れた。PDF assembly／pipelineの検証枠1億workを入力の累積workへ加えるよう修正した。exact成功・1不足拒否、record／spool／output境界、prior伝播と失敗後の拒否を維持し、production driverの上限は変えない。

幅の追加発見ごとのcatalog再構築、source／label反復ごとの再構築は残る。line passが128以内に収まる受入や全巻性能の完了は主張しない。独立child variantの専用数式・画像・rowspan・強制改ページ、固定native/vector body、残る名前指定・柱/footer content・段組・元全巻・公開CLI・manifest・管理ホスト・性能・著者および人手受入は未完である。


### 14.230 独立した子表headerの数式・画像PDF回帰

§14.228〜229の子表header経路へ、native inline／block数式、SVG inline／block数式と式番号、PNG図版、非描画anchorを接続した専用回帰を追加した。子表を親のbodyセルに配置し、親自身も本文段落のheadを持つ。一つの表を親headごと複製するケースとは別に、元table index 0／1の反復variant、各物理fragmentの実measurement、意味数式2件の一度だけの消費、反復数式と画像のArtifact、実font／subset／CID／ToUnicodeを検査する。

本文／脚注 × native／SVG／SVG+PNGの6 casesに、無変更原HaranoでのSVG本文／脚注を加えた8 casesを実driverからPDF化する。幅は100／140／180pt、SVG脚注のみ既存のlist／marker insetを含む120／160／200pt。親head追加分として物理領域の高さを明示的に32pt増やし、元数式・画像metricsは変更しない。狭い100pt脚注で固定SVGが収まらない場合は、子表でもNoFeasibleLineで拒否しcallbackへ到達しないことを要求する。

fixture再構成時のnode採番は、明示式番号の子nodeも含む既存helperを共用する。productionの組版処理・予算・公開profile登録には変更を加えない。この回帰により専用の子表数式・画像検証を補ったが、rowspan・強制改ページvariant、固定native/vector body、残る名前指定・柱/footer content・段組・元全巻・公開CLI・manifest・管理ホスト・性能・著者および人手受入は引き続き未完である。実行結果は[進捗記録](28-vmb-book-production-progress.md#book-2-nested-header-media-design-14230)に記録する。

### 14.231 柱・footerの元sourceから専用flowと字形への接続

ADR-0095に従い、実選択masterのheader／footer内容からBookV2PageRegionTextFlowを準備する。元region、master id、role、body／navigationの正確な参照を保持し、paragraph／headingのnode、text bufferを借用したUTF-8、source span、言語、soft／hard break、ordinary computed styleを共通段落carrierへ渡す。本文側のPreparedBookV2TextFlowとはnavigation型引数を分け、柱の内容を本文として消費できないようにする。region内のpage指定は本文のページ選択へ伝播させない。

新flowのfingerprintは専用domain、完全canonical input、master id、region ownerとHeader／Footer roleを束縛する。同じmasterを別の物理ページで選択してもsource identityを保持する。別bodyや再準備したnavigationは正確な参照照合で拒否する。段落・inline site・eventの保持slotはprior recordsへ加えて確保前に上限確認し、AST・text・ordinary styleの既存制約も維持する。

shape_book_v2_page_region_textは、共通bidi／grapheme／line-context engineへ専用の閉じた入力を追加し、resource-set /3の実TrueType／CFF /2 font instanceで字形を生成する。結果はregion flow、admitted ledger、limits、epoch、実line-contextへ束縛した別の型とし、本文のshape receiptへ変換しない。改行位置はUTF-8／grapheme境界で検証し、言語・fontや本文structure nodeを補わない。

これは柱・footerの元sourceとシェーピングの接続である。物理領域での行選択・高さ／overflow、ページごとの反復Artifact配置、実font／PDF resource closure、driverの累積work／失敗試行費用とPDF接続は次段に残る。内容を未描画のまま捨てないようpage-plan／PDFのUnsupportedPageMaster guardは維持する。専用の原Harano日本語検証・回帰結果は[進捗記録](28-vmb-book-production-progress.md#book-2-page-region-text-design-14231)を参照する。公開profile・全巻・PDF/UAの完了を意味しない。

### 14.232 柱・footerの実領域幅による行選択と高さ検証

[ADR-0096](../adr/ADR-0096-book-2-page-region-lines.md)に従い、BookV2PageRegionInlines／BookV2PageRegionLinesを追加した。元region・body・shape・ledger・limits・epochと、実選択masterを照合して共通inline selectorへ渡す。本文用inline／footnote frameの型には変換しない。段落のstart／end indentを元header／footer矩形の幅から引き、実字形advanceと改行機会で行を選ぶ。行揃えと段落間余白、実line metricsを反映したpage座標のoriginを保持し、元のglyph・source span・強制改行・空段落のblank lineを維持する。外端の段落余白は本文と同様に抑制し、内部の前後余白は加算する。

with_converged_book_v2_page_region_linesは、実選択したsource改行位置でshapeと行選択を反復し、sealed stateが安定してから領域高さを検査する。直接layout関数も高さを検査する。幅不足・改行不能・縦overflowを縮小やclipで隠さず、owner付きerrorで返す。同じmasterを別ページへ配置する際も、配置receiptのpage indexと矩形をfingerprintへ束縛する。

初期breakと各reshapeのcandidate費用を共通allowanceから引き、呼出元の残りpassとeffective limitを守る。保持shape／inline／width scratch／projection／originと前回line contextをrecord予算へ計上する。これはstageの予算接続であり、source準備やshape work、失敗候補を含むdriver全体の累積費用を完成したとは扱わない。

原Harano日本語・controlled TTで幅変更、行揃え、source／字形、強制改行・空段落、高さexact／1単位不足、予算と別ownerの拒否を検証する。結果は[進捗記録](28-vmb-book-production-progress.md#book-2-page-region-lines-design-14232)へ記録する。ページごとの反復Artifact描画、実font／PDF resource closureとdriver接続は残り、未描画regionを捨てないため既存のUnsupportedPageMaster guardを維持する。柱入りPDF、元全巻、公開CLI／manifest、管理ホスト・性能・著者／人手／PDF/UAの受入は未完である。
