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

各ページでは共通境界カーネルの合法な本文候補を列挙し、§14.30の同時適合をすべて評価する。
適合した候補の共通本文境界cost、次いでsource順で決定する。候補列挙上限、ページごとの
評価回数上限、累積work／record上限を超えた場合は失敗し、調べた一部だけを最適解としない。
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
