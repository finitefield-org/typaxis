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

共通cursorの追補（2026-09-06）: `typaxis-pagination/src/production_body.rs`の`paginate_production_body`は、source flowの段落行、block SVG、vector caption、明示改ページを同じcursorで配置する。行とblockのpackage/profile/limits/admission/binding epochを照合し、段落の実行高とblockの実content heightを消費する。paragraphのstart/end indent、start/center/end alignment、before/after spaceを使用する。semantic containerは縦余白と末尾keepを子の最初/最後の配置へ伝え、非ゼロのcontainer indentとnamed-page選択はowner付き保留とする。captionは実際の本文行を消費し、keep_caption=trueではblockからcaption末尾までの実高さを同じページに保つ。keep_with_nextは後続の実行高・余白を含めて判断し、groupが空ページにも収まらない場合はoversizeとする。明示改ページは先頭・連続・末尾を含めて1 nodeにつき必ず次ページを作る。keepと明示改ページの衝突は診断し、片方を黙って無視しない。

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
