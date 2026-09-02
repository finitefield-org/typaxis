# VMB向け組版済み数式ベクター配置の実装設計

## 1. 文書情報

- 状態: Proposed
- 対象: 非公開の`typaxis.contract/1.4` / `typaxis.machine-pdf/production-book-1`
- 主利用者: `texToSvg`等で数式を組版済みのSVGへ変換できるVMB
- 非目標: TypaxisによるTeX解釈、マクロ展開、数式組版、読み上げ文生成
- 関連判断:
  - [ADR-0032](../adr/ADR-0032-semantic-container-and-declared-media.md)
  - [ADR-0033](../adr/ADR-0033-math-safe-vector-and-alternative-binding.md)
  - [ADR-0034](../adr/ADR-0034-document-metadata-language-and-outline.md)
  - [ADR-0035](../adr/ADR-0035-tagged-pdf-structure-and-validation.md)
  - [ADR-0036](../adr/ADR-0036-jpeg-and-opentype-cff-resource-profiles.md)
  - [単位・丸め・座標契約](24-units-rounding-and-geometry.md)
  - [Adobe PDF 1.7 Reference: Form XObjects](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/pdfreference1.7old.pdf)
  - [W3C SVG 1.1: Color](https://www.w3.org/TR/SVG11/color.html)
  - [W3C SVG 1.1: Painting](https://www.w3.org/TR/SVG11/painting.html)
  - [W3C SVG 1.1: Clipping, masking and compositing](https://www.w3.org/TR/SVG11/masking.html)

本書は実装前の設計案であり、current contract 1.3、公開CLI、公開
capability descriptorを変更しない。採用時にはADR-0033の「mathは
SafeVectorを通らない」という判断へ、producer-composed math vectorを
別経路として追加するdecision-gate ADRが必要である。同ADRは、ADR-0034の
closed language-owner/navigation chain、ADR-0035のclosed structure/alternative
mapping、ADR-0036のimmutable SafeVector component/resource-setも同時に
version-upしなければならない。既存`/1` identityの意味を黙って広げることは
できない。

contract 1.4はまだ公開・凍結されていないため、MI4-13より前なら
ADR-0032のprivate staging拡張規則に従って1.4へ追加できる。MI4-13の
公開後に実装する場合は、同じwire shapeを1.4へ後付けせず、新しい
contract/profileを採番する。

## 2. 結論

初版は、VMBが組版した数式を**安全なSVG resource + producer supplied
metrics + source/alternative binding**として受け取り、既存の
Safe-SVG → canonical vector IR → PDF Form XObject経路を拡張して配置する。

採用する主な判断は次のとおりである。

1. TypaxisはTeXを解析・正規化・組版しない。元TeXはopaque UTF-8として
   hashとTextSpanを保持する。
2. inline mathは`math_vector`、block mathは`math_vector_block`という
   新しい明示的nodeにする。既存の`inline_math` / `display_math`から
   暗黙変換しない。
3. SVG bytesはresource catalogからcontained stable readし、required
   SHA-256を照合する。nodeへSVG/XMLを直接埋め込む形式は初版では持たない。
4. metricsはすべて`pdf_point_1_65536`のJSON整数で渡す。binary float、
   CSS unit、ambient font sizeから再計算しない。
5. inline itemの行幅は`advance`、行高は`ascent` / `descent`、実描画位置は
   `origin_x` / `baseline` / `viewport`から決める。
6. block mathは一つのatomic blockで、SVG内部を行・段・pageへ分割しない。
   自動縮小、raster fallback、clipによる成功扱いは行わない。
7. PDFでは同じSVG content keyから一つのForm XObjectを作り、各出現は
   placement matrixとmarked-contentだけを追加する。
8. `currentColor`は呼出側のresolved text paintとして扱う。
9. 一般の1-page PDF fragment importは初版では採用しない。PDFはSVGより
   はるかに広いobject/resource/action/font/filter意味論を持ち、callerの
   safety assertionだけではTypaxisのtrust boundaryを満たさないためである。

この経路は既存のTypaxis-nativeな小さい`typaxis-math`経路と共存できる。
VMB fixtureは表現力の制限を受けない`math_vector`だけを使い、Typaxis-native
mathへのfallbackは行わない。

## 3. 現行実装との差分

現行private 1.4 stagingには、今回利用できる次の部品がすでにある。

- `svg-safe-1`のstable-byte admissionとcanonical SafeVector IR
- vector Figureのintrinsic ratio検査、Display `DrawVector`、PDF Form XObject
- inline mathのatomic item、advance/ascent/descentを用いたlayout概念
- display mathのatomic flow、selected placement、`ActualText`、`/Formula`
- resource、layout、Display、PDF、manifest間のreceipt closure

一方、次が不足している。

- SafeVectorをinline itemとして使うwire/domain
- caller supplied baseline/advance/ascent/descentのvalidated receipt
- `currentColor`と限定opacityを持つsafe SVG profile
- content hashを主keyにしたcross-resource-ID Form XObject deduplication
- block vector mathとequation numberの独立配置
- opaque TeXと組版済みvectorを結ぶbinding
- VMB conversion engine/rules identityを持つmanifest
- capability descriptorのvector kind/metric宣言

既存`svg-safe-1`は意味を変えない。`currentColor`等を追加した入力を
`svg-safe-1`として受理するとADR-0033のparser/IR identityを破るため、
新しいexact media/profile `svg-safe-2`を追加する。

提案するversioned identityは次のとおりである。

| item | identity |
| --- | --- |
| wire media | `svg-safe-2` |
| production SafeVector component | `typaxis.resource-profile/safe-vector/2` |
| production resource set | `typaxis.production-book-resource-set/2` |
| safe SVG parser | `typaxis.safe-svg-parser/2` |
| canonical vector IR | `typaxis.safe-vector-ir/2` |
| vector IR fingerprint | `typaxis.safe-vector-ir-fingerprint/2` |
| vector allocation charge | `typaxis.safe-vector-allocation-charge/2` |
| producer metric validation | `typaxis.precomposed-vector-metrics/1` |
| block vector style registry/cascade | `typaxis.precomposed-vector-style/1` |
| source/vector/alternative binding | `typaxis.precomposed-math-binding/1` |
| producer-composed math block flow | `typaxis.math-vector-flow/1` |
| atomic inline itemization | `typaxis.atomic-vector-inline/1` |
| inline/block selected layout | `typaxis.precomposed-vector-layout/1` |
| vector Display command/receipt | `typaxis.draw-vector-display/2` |
| content-key Form dedupe | `typaxis.vector-form-dedupe/1` |
| per-content vector Form/ExtGState plan | `typaxis.safe-vector-form-plan/2` |
| deduplicated vector Form plan set | `typaxis.safe-vector-form-plans/2` |
| vector PDF object/use closure | `typaxis.safe-vector-pdf-closure/2` |
| SafeVector resource/usage manifest | `typaxis.safe-vector-manifest/2` |
| producer-composed math binding manifest | `typaxis.math-vector-manifest/1` |
| computed language inheritance | `typaxis.computed-language-registry/2` |
| book-navigation profile view | `typaxis.book-navigation-profile-view/2` |
| book-navigation profile receipt | `typaxis.book-navigation-profile-receipt/2` |
| selected metadata/language/navigation state | `typaxis.book-navigation-selected/2` |
| book-navigation PDF observation | `typaxis.book-navigation-pdf/2` |
| book-navigation manifest | `typaxis.book-navigation-manifest/2` |
| PDF/UA-1 production subset | `typaxis.pdfua1-profile/2` |
| production accessibility preflight | `typaxis.production-accessibility-preflight/2` |
| profile-bound lower authorization | `typaxis.production-accessibility-authorization/2` |
| structure role vocabulary | `typaxis.structure-role-vocabulary/2` |
| logical structure registry | `typaxis.structure-registry/2` |
| selected structure binding | `typaxis.selected-structure-binding/2` |
| marked-content plan | `typaxis.marked-content-plan/2` |
| tagged PDF observation | `typaxis.tagged-pdf-observation/2` |
| in-tree tagged PDF validator | `typaxis.tagged-pdf-validator/2` |
| tagged PDF/accessibility manifest | `typaxis.tagged-pdf-manifest/2` |
| release validation policy | `typaxis.pdfua1-validation-policy/2` |
| Matterhorn assessment ledger | `typaxis.matterhorn-assessment/2` |

`safe-vector/2` componentは既存`svg-safe-1`の`/1` parser/IR経路を保存した
まま、新しい`svg-safe-2`の`/2` parser/IR経路を追加する。production resource
setは既存のcomponent順を保ち、SafeVector componentだけを`/2`へ置換する。
対象standardは引き続きPDF/UA-1であり、XMP `typaxis.book-xmp/2`のbytesは
同じmetadata/language入力に対して変えない。ただしclosed semantic domain、
alternative mapping、validation evidenceが増えるため、PDF/UA
profile/policy/assessment identity自体は`/2`にする。

ADR-0034のdocument metadata、BCP 47 parse/canonicalization、UTC timestamp、
outline registry、destination registryはそれぞれ既存`/1`を保つ。一方、languageを
持てるclosed node kindへ`inline_vector`、`math_vector`、`math_vector_block`、
`vector_figure`を追加するため、computed-language registryと、
その完全なowner集合・selected paint・PDF observationをbindするbook-navigation
profile/selected/manifest chainは`/2`にする。`typaxis.book-xmp/2`のserialization
identityは変更せず、同じmetadata/language入力から同じXMP bytesを得る。
`typaxis.book-navigation-pdf/2`は別のuntagged PDFを生成するserializerではなく、
`typaxis.tagged-pdf-observation/2`と同じ最終PDF hashからInfo、catalog `/Lang`、
outline、language paint、`book-xmp/2`を投影したobservationである。

既存`typaxis.basic-block-style-registry/1`、`typaxis.basic-flow-registry/1`、
`typaxis.math-flow/1`、
`typaxis.safe-vector-selected-layout/1`、`typaxis.draw-vector-display/1`、
`typaxis.safe-vector-form-plan/1`、`typaxis.safe-vector-form-plans/1`、
`typaxis.safe-vector-pdf-closure/1`、`typaxis.safe-vector-manifest/1`、
`typaxis.math-manifest/1`、`typaxis.computed-language-registry/1`、
`typaxis.book-navigation-profile-view/1`、
`typaxis.book-navigation-profile-receipt/1`、
`typaxis.book-navigation-selected/1`、`typaxis.book-navigation-pdf/1`、
`typaxis.book-navigation-manifest/1`、`typaxis.tagged-pdf-manifest/1`のcanonical
record、applicable Schema、意味は変更しない。SafeVector manifest `/2`は
content-key alias/dedupeとSafe-SVG 2 factを所有し、math-vector `/1`は二つの
math kindのsource/alternative/metric bindingをSafeVector usageへ結ぶ。tagged
manifest `/2`だけが新しいFormula/Figure/number structureと`/2` accessibility
receiptを受ける。resource-set `/2`のbuildは、`svg-safe-1`を使う既存Figureを
含む全SafeVector usageにSafeVector manifest `/2`を使い、`/1`を混在させない。

accepted SVG syntax、numeric conversion、metric意味、line-break class、spacing
discard、block styleのkind/property applicability/cascade、block flowのowner/
allocation/terminal、block numbering、
alternative mapping、structure role、Form key、limit chargeのいずれかを変える
場合は、対応identityの新versionとcompatibility判断を必要とする。

## 4. Wire contract

### 4.1 Resource declaration

SVGは既存resource catalogの`resources.images`を使う。別の
`VectorResourceId`を追加せず、typed `ImageResourceId`とadmitted hashを
再利用する。resource URIをnodeへ直接書くことは禁止する。

`svg-safe-2` resourceの概念形は次のとおりである。

```json
{
  "expected_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "image_id": 7,
  "media_type": "svg-safe-2",
  "uri": "math/sha256-0123456789abcdef.svg",
  "vector_provenance": {
    "engine_id": "vmb.texToSvg",
    "engine_version": "2026.09.0",
    "rules_version": "vmb.math-safe-svg/1"
  }
}
```

`svg-safe-2` branchでは`expected_sha256`をrequired nonnull、
`vector_provenance`をrequiredにする。wire全体の`expected_sha256` memberは
既存1.4 shapeどおりrequired nullableだが、Schema conditionalが
`media_type = svg-safe-2`のnullを`P1102`で拒否する。`vector_provenance`は
`svg-safe-2`だけで構造上許可し、既存`svg-safe-1`、PNG、JPEG等ではSchemaで
禁止する。これによりmath参照のcross-referenceを解く前に、全Safe-SVG 2
resourceがhash/provenanceを持つ。

production profileのmath-vector二kindは`svg-safe-2`だけを受理する。
generic `inline_vector` / `vector_figure`は`svg-safe-1`または`svg-safe-2`を
参照できるが、`svg-safe-1`をmathとして扱うことは禁止する。既存`figure`の
vector branchは`svg-safe-1`だけという既存意味を保ち、`svg-safe-2`を受理しない。
PNG/JPEG branchもADR-0036の既存規則から変えない。

各provenance stringはnonempty printable ASCII、最大128 bytesとする。
このrecordはproducer assertionであり、Typaxisがそのengineを実行した証明
ではない。parser動作の選択にも使わず、source/vector bindingとmanifestへ
そのままhash-bindする。

同じ`image_id`の重複宣言は従来どおりsyntax `P1102`で不正であり、resource
open前に拒否する。同じcontent SHA-256を異なるIDで宣言することは許すが、
同一hashなのにfull stable bytesが異なる場合はcryptographic collisionとして
`R7100`で失敗する。異なるIDのprovenanceは異なってよく、各alias factへ保持
する。media/parser/IR identityが異なる場合も宣言自体は競合ではなく、9.2の
別`VectorContentKey`として扱う。

### 4.2 Metric record

`math_vector`、`inline_vector`、`math_vector_block`は共通のclosed recordを持つ。

```json
{
  "advance": 1638400,
  "ascent": 589824,
  "baseline": 537395,
  "descent": 262144,
  "origin_x": 0,
  "viewport": {
    "height": 786432,
    "width": 1572864
  }
}
```

数値はすべて既存common Schemaのcanonical JSON safe integerである。
`origin_x`だけはsigned `length`、それ以外は関係に応じてpositiveまたは
nonnegative lengthを使い、すべてchecked `i64`へlowerできなければならない。
単位はすべて`pdf_point_1_65536`であり、record内にunit fieldは置かない。
DocumentPackage rootのsingular `coordinate_unit`とcapability rootのplural
`machine_input.coordinate_units`が、それぞれのartifactにおける唯一の単位
宣言である。
`texToSvg`側のem/ex/CSS px等から変換が必要な場合は、VMBの
`rules_version`がexact rational factorとround-half-to-evenを固定し、package
生成前にこの整数へ変換する。Typaxisはambient font sizeやroot SVGのunit suffix
からnode metricsを再scaleしない。

各fieldの意味は次のとおりである。

| field | 意味 |
| --- | --- |
| `advance` | pen originから次のinline pen originまでの論理進行幅 |
| `ascent` | 本文baselineより上でline boxが確保する量 |
| `descent` | 本文baselineより下でline boxが確保する量 |
| `origin_x` | pen originからSVG viewport左端までのsigned offset |
| `baseline` | viewport上端から数式baselineまでの下向き距離 |
| `viewport.width` | 実際に配置するSVG viewport幅 |
| `viewport.height` | 実際に配置するSVG viewport高 |

`viewport.width` / `viewport.height`が、要望中の表示幅・表示高さを兼ねる。
SVG rootからadmitしたintrinsic width/heightを`Iw` / `Ih`、node metricsを
`Vw` / `Vh`とすると、layout前に次を検証する。

```text
advance > 0
ascent > 0
descent >= 0
Vw > 0
Vh > 0
0 <= baseline <= Vh
ascent >= baseline
descent >= Vh - baseline
```

続いて`s = round_half_even(Vw * 65536 / Iw)`をchecked `i128`で一度だけ
求め、`s`が既存`positive_unitless_16_16`へ収まることと、同じ`s`で`Iw`と`Ih`を
scaleして個別にround-half-to-evenした結果が`Vw` / `Vh`になることを検証する。
これが配置時の唯一のuniform scaleであり、nonuniform scaleとx/y別scale導出を
禁止する。丸め済みLength同士の`Vw * Ih == Vh * Iw`は要求しない。同じscaleを
使った正しい結果でも各軸の最終丸めによってこの積が異なり得るためである。

`origin_x`はsigned safe integerであり、`origin_x + Vw`をcheckedに計算
できなければ不正である。viewportが`advance`から少しoverhangすることは
font glyphと同様に許す。行幅計算は要求どおり`advance`を使い、viewport
boundsはframe外paintの検出にだけ使う。

### 4.3 Inline nodes

一般inline vectorとmath vectorを別kindにする。これによりtagged PDFで
`/Figure`と`/Formula`を推測せずに選択できる。

```json
{
  "actual_text": null,
  "alt": "2分の1",
  "image_id": 7,
  "kind": "math_vector",
  "language": "ja",
  "metrics": {
    "advance": 1638400,
    "ascent": 589824,
    "baseline": 537395,
    "descent": 262144,
    "origin_x": 0,
    "viewport": {"height": 786432, "width": 1572864}
  },
  "node_id": 12,
  "source_tex": {
    "text_span": {"end_byte": 11, "start_byte": 0, "text_id": 3}
  },
  "spacing": {"after": 16384, "before": 16384},
  "span": {"end_byte": 111, "source_id": 0, "start_byte": 100}
}
```

- `source_tex.text_span`はexact UTF-8の元TeXを指す。Typaxisはそのbytesを
  parse、trim、Unicode normalize、delimiter除去、再formatしない。
- `source_tex.text_span`を覆うTextMapはexactly one identity mappingであり、
  対応SourceSpanはowner nodeのSourceSpan内に含まれなければならない。
  inlineでは通常owner spanと同一、番号付きblockではformula subspanと
  equation-number subspanを重複しない別mappingにする。
- source bytesはnonempty UTF-8、BOM/NULなしとし、LF/TABを含むTeXは
  byte-exactに保持する。
- `alt`はrequiredで、Unicode 16.0 `White_Space`以外を少なくとも一scalar
  含み、C0/C1 controlを含まないtextとする。trim、Unicode normalize、
  whitespace collapseを行わず、`/Formula /Alt`にexact使用する。
- mathの`actual_text`はrequired nullableとする。nonnullならPDF
  `/ActualText`にexact使用し、nullなら`alt`をresolved `/ActualText`として
  exact使用する。nonnull値にも`alt`と同じmeaningful/control規則を適用し、
  暗黙にTeXを使わない。
- `language`はADR-0034と同じoptional overrideであり、TeX dialectではなく
  読み上げ文の自然言語である。BCP 47 parse/canonicalizationは既存
  `typaxis.bcp47-language/1`を使い、4つのnew kindを含むinheritance recordは
  `typaxis.computed-language-registry/2`だけが発行する。
- `spacing.before` / `spacing.after`はnonnegative Lengthである。

`inline_vector`は同じ`image_id`、`metrics`、`spacing`、`alt`、
`actual_text`、`language`を持つが、`source_tex`を禁止する。semantic roleは
Figureである。decorative inline vectorは初版のproduction profileでは
受理しない。`inline_vector.actual_text`のnonnull値はpaint-level
`/ActualText`へexact使用するが、nullはFigure policyどおりabsenceを意味し、
`alt`へfallbackしない。`inline_vector`と後述`vector_figure`の`alt`、および
nonnull inline-vector `actual_text`にもmathと同じmeaningful/control規則を
適用する。

### 4.4 Block nodes

block mathの概念形は次のとおりである。

```json
{
  "actual_text": "複数行の連立式",
  "alt": "複数行の連立式",
  "classes": ["display-math"],
  "equation_number": {
    "minimum_gap": 65536,
    "node_id": 21,
    "span": {"end_byte": 264, "source_id": 0, "start_byte": 261},
    "text_span": {"end_byte": 3, "start_byte": 0, "text_id": 5}
  },
  "image_id": 8,
  "kind": "math_vector_block",
  "language": "ja",
  "metrics": {
    "advance": 3932160,
    "ascent": 1048576,
    "baseline": 983040,
    "descent": 524288,
    "origin_x": 0,
    "viewport": {"height": 1507328, "width": 3932160}
  },
  "node_id": 20,
  "source_tex": {
    "text_span": {"end_byte": 60, "start_byte": 0, "text_id": 4}
  },
  "span": {"end_byte": 264, "source_id": 0, "start_byte": 200}
}
```

`math_vector_block`の`source_tex`、`alt`、`actual_text`、`language`は4.3の
math規則をそのまま使う。`actual_text`はrequired nullableで、null時だけ
`alt`をresolved `/ActualText`として使う。`equation_number`もrequired
nullableとし、nullは番号を持たないことを明示する。省略とnullを同義にせず、
unknown memberを許さない。
blockのeffective languageは`alt`、resolved `actual_text`、equation-number textへ
適用する。`equation_number`自身は`language` memberを持たず、第五のlanguage-
override ownerにはならない。そのtext/structure childは親`math_vector_block`の
computed language fingerprintを参照する。opaque `source_tex`のdialectやparse
ruleは選択しない。

左寄せ・中央寄せ・右寄せ、前後余白、indent、named page、
`keep_with_next`は重複fieldを増やさず、既存typed styleの`text_align`、
`space_before`、`space_after`、`start_indent`、`end_indent`、`page`、
`keep_with_next`を使う。wire keywordは既存契約どおり`start|center|end`であり、
production profileのhorizontal/LTRではleft/center/rightへ対応する。

style selector block typeへ`math_vector_block`と`vector_figure`を追加する。
この二kindは`typaxis.precomposed-vector-style/1`のclosed registryで解決し、
既存`typaxis.basic-block-style-registry/1`へkindを追加しない。property value型、
`important` / specificity / source-order / `extends` precedence、inheritance計算は
既存cascadeを再利用するが、新registry identityを持つcomputed receiptだけが
new kindへ適用できる。
両kindで`space_before`、`space_after`、`start_indent`、`end_indent`、
`text_align`、`page`、`keep_with_next`を適用し、`vector_figure`だけはさらに
`keep_caption`を適用する。`math_vector_block`の`font_family`、`font_size`、
`line_height`はequation-number textのshapeとline boxにだけ適用し、producer
metricsやSVG scaleを変更しない。`vector_figure` ownerの同3 propertyは既存
Figure同様inapplicableで、caption childが自身のstyleを解決する。`width`は
両kindでinapplicable `L5101`とし、resizeしたいproducerは新しいuniformly-
scaled metricsまたはviewportを明示する。`keep_caption`は
`math_vector_block`ではinapplicable `L5101`である。

`equation_number`はrequired nullableである。存在する場合は独立NodeId、SourceSpan、
TextSpanを持つ通常textとしてshapeし、そのTextSpanは`source_tex`と重ならない
identity mappingを持つ。数式SVG、数式`ActualText`、vector resource hashには
含めない。Typaxisは番号を生成・increment・localizeせず、producerがTextSpanで
渡したexact textだけを使う。NodeIdはownerを含む全source/generated NodeIdと
異なり、
`minimum_gap`はpositive Lengthでなければならない。number TextSpanはnonempty
UTF-8でUnicode 16.0 `White_Space`以外を少なくとも一scalar含み、C0/C1 controlを
含まない。既存text pipelineで一つのnonwrapping line boxへshapeし、breakや
複数lineへのfallbackを許さない。selected shapeの`Nw` / `Nh`はpositiveで
なければならず、満たさない場合は`L5100`である。

番号配置は次に固定する。

- horizontal positionはinner frameのlogical end。
- vertical positionは数式viewportのvertical centerと番号line boxのcenterを
  一致させる。
- 数式本体の`text_align`は番号の有無にかかわらずinner frame全幅に対して
  計算する。
- 数式viewportと番号rectangleの間にrequired positive `minimum_gap`を置く。
- 二つのrectangleが交差する場合は`L5100`。数式のcenterをずらす、番号を
  SVGへ合成する、改行する、縮小するfallbackはない。

番号line box幅/高を`Nw` / `Nh`、viewport高を`Vh`とすると、番号ありの
block content高`Bh`は`max(Vh, Nh)`である。`equation_number = null`では
`Nw` / `Nh`と番号rectangleは存在せず、`Bh = Vh`とする。各rectangleのtop offsetは
`round_half_even((Bh - child_height) / 2)`とし、残るodd unitはblock-end側へ
置く。paginationとpaint/structure occurrence boundsはこの同じ`Bh`を使い、
番号がviewportより高い場合もblock boundsからはみ出さない。paintとstructure
child orderはいずれもformula、equation numberの順に固定する。

一般block vectorは`vector_figure` kindとし、次のclosed shapeを持つthin
Figure variantとする。

```json
{
  "alt": "部品構成図",
  "caption": [],
  "classes": ["diagram"],
  "image_id": 9,
  "kind": "vector_figure",
  "language": "ja",
  "node_id": 30,
  "span": {"end_byte": 320, "source_id": 0, "start_byte": 300},
  "viewport": {"height": 3932160, "width": 7864320}
}
```

profileはresourceのadmitted mediaがvectorであることとintrinsic ratioを
layout前に証明でき、raster Figureとcapabilityを混同しない。
`vector_figure`は非数式のFigureであり、`source_tex`、`actual_text`、
`equation_number`を禁止する。TeXから生成され、元TeXを保持してFormulaとして
読み上げる可換図式は`vector_figure`ではなく`math_vector_block`を使う。
effective languageは`alt`へ適用し、既存Figureと同じくcaption childの
language-inheritance parentになる。
`vector_figure.viewport`は4.2と同じ一つの`positive_unitless_16_16` scaleで
intrinsic width/heightを個別roundした結果とexact一致させる。baseline、advance、
originは持たず、aligned viewport rectangle自体がblock paint geometryである。
`math_vector_block.metrics.advance`はmanifest/bindingへ保持するが、blockの
alignment/overflow geometryにはviewport widthを使う。blockのpaint/pagination
heightは4.4の`Bh`であり、inline line box用の`ascent` / `descent`へ置換しない。

## 5. Baselineと配置式

Typaxis内部layout座標はtop-left / Y-downである。inline pen originを
`pen_x`、本文baselineを`line_baseline_y`とすると、SVG viewportの左上は
次である。

```text
viewport_left = pen_x + origin_x
viewport_top  = line_baseline_y - baseline

line_baseline_y = viewport_top + baseline
```

したがって、SVG下端ではなくproducer supplied baselineが本文baselineへ
一致する。resource SVGの`viewBox min-x/min-y`はSafeVector Form plan内で処理
し、この`origin_x` / `baseline`を二重適用しない。

blockでは先にalignmentから`viewport_left`を決め、
`pen_x = viewport_left - origin_x`を導出する。manifestとDisplayはpen origin、
viewport rectangle、baseline、uniform scale、最終placement matrixをすべて
bindする。

## 6. Inline itemizationと改行

### 6.1 Atomic item

`math_vector`と`inline_vector`は内部break candidateを持たない一つの
`AtomicVectorInlineItem`へlowerする。SVG path、subpath、glyph-like outlineを
line breakerへ公開しない。

初版のhorizontal/LTR profileでは、line-break boundary classを`AL`、bidiを
atomic LTR isolateとする。itemizerはsource textへU+FFFC等を挿入せず、
line-break classifierへsource provenance付きのsynthetic `AL` unitを一つ渡す。
隣接する日本語・句読点とのbreak可否は、そのunitを含む完全なlogical unit
列に対する既存Unicode ruleとJapanese pair tableで決める。Japanese tailoringは
Unicode candidateを追加せず、既存どおり禁止だけを強める。たとえばclosing
punctuationをline頭へ送る独自breakをvector側から追加しない。

### 6.2 Widthとspacing

line breakerが使うlogical widthは次である。

```text
same-line previous contentがある場合: spacing.before
atomic box:                           advance
same-line next contentがある場合:     spacing.after
```

spacingは裸の`Glue`としてbreak candidateを増やしてはならない。各vector
境界を、Unicode/Japanese ownerが決めた`BreakKind` / penaltyと、同一line時
だけ有効な`same_line_width`を持つ`VectorBoundaryItem`へlowerする。breakを
選んだbranchのpre/post widthはzero、no-break branchだけが指定spacingを持つ。
既存item型で表現できない場合は専用variantを追加し、許可・禁止・mandatoryを
数値penaltyだけで近似しない。

vector境界ではJapanese pair tableのpermission/penaltyだけを使い、その
`natural_gap` / stretch / shrinkを指定spacingへ重ねない。したがって
`spacing.before` / `spacing.after`は各側のexact total gapで、常にzero-stretch /
zero-shrinkである。vectorがline頭なら`before = 0`、line末なら`after = 0`とし、
境界で改行した場合はその境界gapを両lineでzeroにする。これにより指定字間を
保ちながら、spacing自体が括弧・句読点の禁則を破るbreakを作らない。

二つのinline vectorが隣接する場合は、左の`after`と右の`before`を加算する。
itemizerは一つのlogical boundaryにつきexactly one `VectorBoundaryItem`を
発行し、二つのboundary itemやその間のbreak candidateを作らない。producerは
不要な側をzeroにする。spacingをSVG viewBoxや`advance`へ暗黙合成しない。

### 6.3 Line metrics

各候補lineについて、text runと全atomic objectから次を求める。

```text
content_ascent  = max(text_ascent,  each_vector.ascent)
content_descent = max(text_descent, each_vector.descent)
content_height  = content_ascent + content_descent
extra_leading   = max(0, computed_line_height - content_height)
leading_before  = round_half_even(extra_leading / 2)
leading_after   = extra_leading - leading_before
line_height     = leading_before + content_height + leading_after
line_baseline_y = line_top + leading_before + content_ascent
```

このline heightをpaginationのline advanceに使う。高い分数、総和、積分、
添字、行列が同じlineにあっても、次行は最大ascent/descentの外へ進む。

`ascent >= baseline`かつ`descent >= viewport.height - baseline`をsyntax
preflightで証明するため、SVG viewport全体はline box内に入る。

### 6.4 Frame fit

break costとpen advanceには`advance`を使うが、最終line feasibilityでは
`origin_x .. origin_x + viewport.width`もframe boundsへ照合する。現在lineで
fitせずempty next lineでfitする場合はvector全体を次lineへ移す。empty line
でもlogical advanceまたはvisual viewportがfitしない場合はterminal
`L5100`である。動的`line_height`がcurrent frameの残り高へ入らずempty next
frameへ入る場合もline全体を次frameへ送り、empty full frameにも入らなければ
`L5100`とする。高いvectorだけを前のpageへpaintすることはない。

## 7. Block layoutとpagination

`math_vector_block`は一つのatomic block / one-terminal flowとして扱う。
各ownerは`typaxis.math-vector-flow/1`配下で、native `display_math`の
`MathFlowId`とはnominalにも採番空間にも異なるdense `MathVectorFlowId`を一つ持つ。
`typaxis.math-flow/1`へnew kindを追加せず、native mathのID、record、golden bytesを
変えない。registryはvalidated Documentの`math_vector_block` NodeId preorderを
worker起動前に走査し、0から連続採番する。caller登録順、hash-map順、page、
paint順、worker完了順は採番へ影響しない。

各flow recordは`MathVectorFlowId`、owner NodeId、親の`FlowId`とposition、
`ValidatedMathVectorReceipt` fingerprint、computed style fingerprint、LayoutEpoch、
exact terminal `1`をbindする。親flowはtyped `math_vector_block` itemを一つ消費し、
このterminalを選択済みplacementが満たした後だけ次positionへ進む。page送りは
未消費の同じflowを次frameで評価することであり、空fragmentや第二fragmentを
発行しない。SVG内の`aligned`、matrix、可換図式、複数pathをTypaxisのline/
fragmentへ展開しない。

productionの親flow registryでは`math_vector_block`を既存のatomic display-math
item categoryへ、`vector_figure`を既存Figure item categoryへ投影し、exact wire
kindはvalidated document/domain receiptと`typaxis.precomposed-vector-layout/1`に
保持する。`math_vector_block`のsource/alternative bindingはさらに
`typaxis.precomposed-math-binding/1`へ保持する。これはMI4-13前のprivate
`typaxis.semantic-container-flow-registry/1`をADR-0032のstaging規則で完成させる
場合に限る。既存`typaxis.basic-flow-registry/1`のowner/content vocabularyは
広げない。MI4-13後に採用する場合は1章の新contract/profileとともに、production
flow registryおよびselected-container closureも新versionにする。

layout順は次のとおりである。

1. computed styleから前後space、indent、alignment、page、keepを確定する。
2. inner frame widthをchecked計算する。
3. SVG viewport widthと`Bh`を検査し、番号が存在する場合だけequation number
   widthとminimum gapも検査する。
4. `text_align`でformula viewport左端を決め、numberを独立配置する。formula
   topとnumber topは4.4のcenter ruleから求める。
5. 既存block-spacing ownerが選んだ`effective_space_before + Bh`がcurrent frameに
   入らず、`Bh`がempty next frameに入るならpending glueをboundaryで消費し、
   block全体を次page/columnへ送る。
6. `Bh`がempty full frameにも入らない、またはwidthを超える場合は`L5100`。

前blockの`space_after`と当該blockの`space_before`は既存契約どおりcollapse
せずchecked加算する。`space_before`はpage/column頭で抑制し、pending glueは
boundaryを越えてcarryせず、`space_after`だけで新しいpageを作らない。
`keep_with_next` groupも既存typed policyをそのまま使い、fitしないときに
暗黙解除しない。SVG内部は決して分割しない。producerが明示的な分割位置を
必要とする場合は別々の`math_vector_block`と既存`page_break` /
`keep_with_next`を使う。

`vector_figure`もviewport paintを一つのatomic Figure imageとして扱い、
viewport内部を分割しない。captionは既存Figureの独立caption flow、
`keep_caption`、source-order paint/structure policyを再利用する。

初版のoverflow policyは常に`error`である。fit-to-width、shrink-to-fit、
crop、rasterize、page回転はcapabilityとしてadvertiseしない。

## 8. Safe SVG 2

### 8.1 基本subset

`svg-safe-2`はADR-0033の`svg-safe-1`を基礎にし、次だけを追加する。

- paint value `currentColor`
- `fill-opacity`、`stroke-opacity`のscalar paint alpha
- alphaを保持するcanonical IR fieldとPDF ExtGState plan

`fill-opacity` / `stroke-opacity`はXML presentation attributeとして`g`とpaint
geometryだけで許可し、既存paint propertyと同じsource nestingでinheritして
各drawへresolved値を保存する。`style` attributeやCSS declarationとしての
指定、およびclip geometryでの指定は禁止する。初期値は両方exact 1で、childの
specified値はinherited値を置換し、親子値を乗算しない。

それ以外のelement、path、viewBox、transform、clip、fixed-point、limit、
external-reference禁止規則は`svg-safe-1`を継承する。特に次は引き続き
terminal errorである。

- `script`、event attribute、animation、`foreignObject`
- `image`、embedded raster、font、`text`、`tspan`
- CSS、`style`、selector、media query
- `href`、XLink、external/data/file/network reference
- `use`、symbol、marker、gradient、pattern、mask、filter、blend mode、`opacity`
- entity、DOCTYPE、processing instruction、unknown element/attribute

一般的な`texToSvg`出力が`defs/path/use`を使う場合、VMBの
`rules_version`でpathを事前展開して`svg-safe-2`へlowerする。Typaxisは
unsupported raw SVGを便利のために解釈せず、明確な`R7100`で拒否する。

### 8.2 currentColor

IRはpaintを`None | FixedRgb8 | CurrentColor`として保持する。math vectorの
`currentColor`は、そのnodeを所有するparagraph/blockのresolved text paint
へbindする。現行production targetでauthored colorを持たない場合、その
resolved valueはexact blackである。

新しいpaint lexicalは`fill`または`stroke` value全体がexact ASCII
`currentColor`の場合だけである。case alias、`inherit`、`var()`、前後の
whitespaceを持つ値は受理しない。

generic inline/block vectorも同じplacement paint ownerを使う。text paintを
authorできない現行style domainではexact blackとなり、将来color propertyを
追加する場合はそのstyle/receipt identity更新なしに解決元を変えない。

SVG側の`color` attribute/propertyは受理しない。したがってresource内部で
`currentColor`を別値へ再定義するbranchはなく、唯一の解決元はplacementの
resolved text paintである。

Form stream内で`CurrentColor` drawはcolor operatorを発行せず、Form呼出時の
nonstroking/stroking colorを継承する。各drawを`q ... Q`で隔離するため、
同じForm内の`FixedRgb8` drawが後続`CurrentColor`へ漏れない。page contentは
`q`の内側で`Do`の直前にresolved text colorをstroking/nonstrokingの両方へ
設定し、`Do`後に`Q`で復元する。

これにより、同じmath Form XObjectを異なる本文色から再利用できる。
resolved colorはplacement receiptへ含めるが、Form dedupe keyへは含めない。

### 8.3 Opacityとclip

alpha lexicalはexact `0`、`1`、`0.` + 1〜6桁、または`1.` + 1〜6個の`0`だけを
受理し、sign、`.5`、`1.`、leading zero、exponent、前後whitespaceを拒否する。
IRでは`fill-opacity`と`stroke-opacity`を個別のunsigned 16.16へ
round-half-to-evenする。SVGのgroup/object `opacity`は、overlap済みgroupを
offscreen compositingする意味を持ち、paint alphaの単純な乗算とは一致しない。
そのため初版では明示的なunsupported featureとして拒否する。mask、soft
mask、blend mode、isolated transparency groupも許さない。

PDFは`(fill_alpha, stroke_alpha)`ごとに一つのExtGStateを作り、Form-local
resource nameを値の昇順で割り当てる。ambient alphaを継承しないよう、
`(1, 1)`を含む全drawがresolved pairのExtGStateを明示してからpaintする。
各dictionaryは`/Type /ExtGState`、fill用`/ca`、stroke用`/CA`だけをこの機能から
追加し、`/BM`、`/SMask`、`/AIS`等を出さない。
enabled fillまたはenabled strokeのalphaがpositiveなdrawを一つも持たない
resourceは空描画として拒否する。

clipは既存SafeVectorのlocal `clipPath` subsetだけを使い、root viewport clip
を常に最外周に置く。unknown/unused/cyclic/forward/external clip referenceは
拒否し、PDF path `W` / `W*` + `n`へ決定的に変換する。

### 8.4 Limitsとone-time charge

新しい無制限budgetは追加せず、既存M4 limit ownerへ次のように課金する。

| work | limit / refusal code |
| --- | --- |
| encoded SVG bytes | per-resource `max_image_bytes` + aggregate `max_resource_bytes` / `R7100` |
| SVG element/path/depth | `max_vector_nodes` / `max_vector_path_segments` / `max_vector_nesting_depth` / `R7120`〜`R7122` |
| canonical IR allocation | `64 * nodes + 80 * stored_segments + 48 * paint_or_clip_commands + source_clip_id_bytes`をchecked計算して`max_decoded_image_bytes`へ課金 / `R7111` |
| `source_tex` | 参照先TextBufferは既存admissionで一回だけ`max_text_buffer_bytes` / aggregate `max_text_bytes`へ課金し、slice長だけをper-buffer上限へ再照合 / `T2100`、aggregateへ再加算しない |
| `alt`とnonnull `actual_text` | 各authored stringをper-buffer上限へ照合し、各一回だけaggregate `max_text_bytes`へ加算 / `T2100`・`T2101`。mathのnull fallbackは`alt`のaliasで再課金せず、inline Figureのnullは文字列を生成しない |
| explicit/computed `language` | ADR-0034どおりraw/canonical spellingと各language-capable NodeIdのcomputed valueを一回だけ`max_text_buffer_bytes` / aggregate `max_text_bytes`へ課金 / `T2100`・`T2101`。`/2` registryへの移行でresetしない |
| semantic vector/equation-number nodesとmath-vector block flow owner | 既存Document semantic count/depthの`max_ast_nodes` / `max_ast_nesting_depth` / `P1120`・`P1121`。各flowは対応するadmitted `math_vector_block` nodeにexactly oneで、別のAST chargeを加えず、registry countをそのnode数および`max_ast_nodes`以下とallocation前に照合する |
| selected inline/block vector occurrence | containing fragmentとは別のexplicit auxiliary recordを`max_fragments`へ各occurrence一回 / `L5110` |
| Form、ExtGState、page resource/object | issue前に`max_pdf_objects` / `G6100` |
| Form plan/page spool/serialized bytes | ownerの同時live payloadへ`max_spool_bytes`、次のoutput writeへ`max_output_bytes` / 既存owner code |

同じcontent keyを再利用してもencoded source bytesとcanonical IR chargeを
resource declarationごとに免除しない。一方、Form plan/XObjectはdedupe後の
一つだけをPDF object budgetへ課金する。TeX/alternative、selected occurrence、
spool/outputの別owner chargeと混同せず、limitをpagination retryや別phaseで
resetしない。

## 9. Vector PDF生成とresource deduplication

### 9.1 Form plan

resource admission後のclosed chainは次である。

```text
declared svg-safe-1 or svg-safe-2 (+ svg-safe-2 required SHA-256/provenance)
  -> stable bytes + bounded parser
  -> SafeVector attestation + canonical IR fingerprint
  -> node geometry/metrics binding
  -> selected inline/block placement
  -> Display DrawVectorResource
  -> frozen Form plan
  -> PDF Form XObject + page-local Do usage
```

`typaxis.draw-vector-display/2`のlogical commandはusage ID、owner/kind、image ID、
content key、IR fingerprint、selected placement fingerprint、page/frame/paint
ordinal、viewport rectangle、uniform scale/matrix、resolved currentColorを持つ。
inline/math usageではpen originとbaseline receiptもbindする。resource URI、raw
SVG、raw TeX、PDF object/nameは持たない。

PDF backendはSVG/XMLを再parseしない。Form `/BBox`はadmitted intrinsic
viewportを使い、path、fill、stroke width、line cap/join、clip、alphaをvector
operatorとして出す。正常経路にimage raster XObjectは存在しない。

placementはinternal top-left座標で
`translate(viewport_left, viewport_top) * uniform_scale(s)`を持ち、PDF backend
はdocs/24のpage-root Y flipを一度だけ適用する。SVG `viewBox` mapping、node
placement、page-root変換のどこにも二重Y flipを入れない。

### 9.2 Dedupe key

現行のlogical `ImageResourceId`主導plan keyを、vectorについて次へ変更する。

```text
VectorContentKey = (
  source_sha256,
  media_type,
  safe_svg_parser_id,
  vector_ir_id,
  vector_ir_fingerprint
)
```

`source_sha256`はstable-read full bytesからTypaxisが計算したadmitted hashであり、
callerの`expected_sha256`文字列を未照合のままkeyへ使わない。

同じkeyはresource ID、NodeId、page、resolved currentColorに関係なく一つの
Form plan/XObjectだけを持つ。異なるsource hashが偶然同じIRへlowerしても、
provenance保持のため初版ではdedupeしない。

producer provenanceはForm paint semanticsを変えないためkeyへ含めない。
同じbytesを異なるengine/rulesが生成したという複数のalias assertionは、各
`image_id`のmanifest/receiptへ残したまま一つのFormを共有できる。

要件の「同じSVG hash → 同じXObject」は、同じmedia/parser/IR identityを持つ
`VectorContentKey`内で成立する。全`math_vector`は`svg-safe-2`へ閉じるため、
同じmath SVG hashなら必ずこの条件を満たす。同じbytesを`svg-safe-1`と
`svg-safe-2`の異なるdeclarationで与えた場合は、異なるadmission semanticsを
一つのFormへ混ぜず、別keyとする。

Form object/resource nameは`VectorContentKey`のlexicographic byte orderで
割り当てる。比較はtuple componentごとに、32-byte source hash、media UTF-8、
parser ID UTF-8、IR ID UTF-8、32-byte IR fingerprintの順で行い、曖昧な文字列連結を
keyにしない。hash map insertion、first use page、worker completion、resource
catalog alias順をobject orderに使わない。

manifestはlogical `image_id`からcontent keyへのalias set、aliasごとのconditional
provenance（`svg-safe-2` required、`svg-safe-1` absent）、conditional Form object、
placement count、各usageのmatrixを記録する。同じhashを10回配置したtestは
1 Form + 10 usageでなければ失敗する。
content keyのselected placement countがzeroならresource/alias factは残すが、
Form plan/object/resource nameと`Do` usageは生成しない。positiveならこれらは
requiredであり、total countに加えてalias別countも記録する。

## 10. Source・alternative・accessibility binding

syntax/layout binding ownerは`typaxis.precomposed-math-binding/1`のopaque
`ValidatedMathVectorReceipt`を発行する。base keyは少なくとも次を含む。

- contract/profile/package/session/effective-limit/LayoutEpoch identity
- NodeId、inline/block kind、SourceSpan、language
- exact TeX TextSpan、参照TextBuffer SHA-256、exact slice UTF-8 SHA-256
- exact `alt` / resolved `actual_text` UTF-8 SHA-256
- image ID、URI、stable SVG SHA-256、declared/attested media
- producer engine ID/version/rules version
- Safe-SVG parser/IR IDとIR fingerprint
- 全metric、spacingまたはcomputed block style、resolved currentColor

downstream factをbase keyへ先取りしない。selected-state ownerは
`typaxis.precomposed-vector-layout/1`のreceiptでbase keyを参照し、selected
page/frame/line/block/paint ordinal、viewport rectangle、baseline、matrixを追加する。
`math_vector_block`ではさらに`typaxis.math-vector-flow/1`のflow fingerprint、
`MathVectorFlowId`、親FlowId/position、terminalをexact照合する。inline二kindと
`vector_figure`にはこれらのflow memberを置かない。
Display `/2`はそのselected fingerprintを参照し、resource finalizer/PDF `/2`は
Display fingerprintからForm content key/object/use observationを追加する。structure
`/2`は同じselected paintをowner、role、`Alt`、`ActualText`、`Lang`へ結ぶ。
三つのmanifest projectionがこのchainを双方向に閉じる。upstream receiptへ
downstream object number、MCID、StructureNodeIdを格納しない。

Typaxisが証明するのは「このsource/alternative/metricsが、このhashのvectorと
このplacementへ取り違えなく結ばれた」ことである。TeXとSVGが数学的に
同値か、読み上げ文が言語的に正しいかは検証しない。

PDF mappingは次に固定する。このmappingはADR-0035の`/1` mappingを広げるため、
3章に列挙したstructure/marked-content/validator `/2` identityが必須である。

| node | structure | marked content |
| --- | --- | --- |
| `math_vector` | one `/Formula` | outer Formula MCR + inner property-only `/Span` around the `Do` |
| `math_vector_block` | one `/Formula` | outer Formula MCR + inner property-only `/Span` around the `Do`; optional number child follows |
| `inline_vector` | one `/Figure` | outer Figure MCR; nonnull `actual_text`またはpaint-level `Lang`適用時にinner property-only `/Span` |
| `vector_figure` | one `/Figure` | existing Figure paint policy; paint-level `Lang`適用時はinner property-only `/Span` |

`/Formula /Alt`と`/Figure /Alt`はexact `alt`、marked-content
`/ActualText`はkind別に4.3で定めた値を使う。各Formula/Figure structure elementは
ADR-0034を拡張した`typaxis.computed-language-registry/2`のcomputed languageを
記録し、nearest structure parentのeffective
languageと異なる場合だけstructure `/Lang`を出す。paint-level `/Lang`も
ADR-0035のleaf ruleどおり、
document languageと異なる場合に後述のinner Spanへ出す。mathをdecorative
Artifactへ落とすことは禁止する。

page contentの外側はstructure type + `/MCID`を持つ一つのMCRである。その内側に
MCIDを持たない`/Span << /ActualText ... /Lang ... >> BDC`を置き、同じpaintだけを
覆って内側から閉じる。`/Lang`は適用時だけ出す。再利用Form stream自身には
MCID、`Alt`、`ActualText`、`Lang`を置かず、page-level `Do`だけをこの二重scopeへ
入れる。`inline_vector.actual_text = null`と`vector_figure`はADR-0035のFigure
policyどおりpaint-level `ActualText`を出さないが、computed languageがdocument
languageと異なるplacementは`/Lang`だけを持つinner Spanを出す。

equation numberはFormulaの`/K`でvector MCRに続くsource-owned `/Span` childとし、
その通常text MCRをformula直後のreading orderへ置く。numberをformula
`/ActualText`へ重複合成せず、number childは自身のTextSpan、computed language、
glyph/extraction receiptを使う。

元TeXのexact bytesはDocumentPackage TextStoreとbuild manifestのspan/hash
closureで保持する。初版は非標準PDF dictionary keyや添付fileとしてTeXを
埋め込まない。PDF自体には標準的な`Formula`、`Alt`、`ActualText`、`Lang`
だけを出す。

## 11. Determinismとbuild manifest

同じpackage bytes、resource bytes、fonts、Typaxis binary、profile、limitsから
同じPDF/sidecar bytesを得るため、次を禁止する。

- float geometryまたはplatform SVG renderer
- locale/timezone/environment依存のnumber/color/source処理
- filesystem/network/font lookupを行うSVG feature
- first-use順のForm/ExtGState/object allocation
- source TeXからのengine-side alternative生成
- unsupported featureのwarning-only omission

`typaxis.safe-vector-manifest/2`はresource factと全vector kindの共通placement /
PDF usage factを所有する。`typaxis.math-vector-manifest/1`はmathだけのsource、
alternative、metric bindingをexact SafeVector usage fingerprintへ結び、
`typaxis.tagged-pdf-manifest/2`はSafeVector manifest/usage fingerprintと、
top-levelのmath-vector manifest fingerprintを参照する。さらにmathの各structure
factだけが対応するmath binding fingerprintを参照し、generic vector factには
math参照を置かない。
逆向き参照は作らず、SafeVector → math-vector → tagged-PDFのacyclic dependency
orderにする。
同じproduction branchの既存book-navigation record/fingerprintは`/1`を受理せず、
4 kindを含む`typaxis.book-navigation-manifest/2`へ置換する。そのmanifestは
computed-language `/2`、selected-navigation `/2`、book-navigation PDF observation
`/2`を参照し、tagged-PDF `/2`から逆参照されない。
versioned contract-1.4 build-manifest Schemaのproduction branchは、既存
book-navigation pairをこの`/2` pairへ置換したうえで、それとは別に三つのrequired
vector-chain record/fingerprint pairを各fieldとして束ねる。別のunversioned集約
recordは作らない。この三つはSafeVector `/2`、producer-composed math-vector `/1`、
tagged-PDF `/2`のchainであり、既存Typaxis-native math用
`typaxis.math-manifest/1`のconditional record/fingerprintを置換しない。
このnonnull requirementは`status = built`のcomplete production buildに適用する。
`status = failed`では三つのrecord/fingerprint pairをrequired nullableとし、各pairは
both-nullまたはboth-nonnullだけを許す。failure時点までに対応ownerがcomplete
recordを発行済みならそのrecordとfingerprintを保持し、未到達phaseのrecordを
空配列や合成fingerprintで捏造しない。`status = built`では三pairすべてをnonnullに
固定する。
`status = built`でSafeVector declarationがzeroならresource/usage array、
math-vector usageがzeroならmath-vector fact arrayをcanonical emptyにするが、
該当するSafeVector/math-vector recordとfingerprintはomitまたはnullにしない。
admitted resourceがunusedなら、
そのSafeVector resource factだけを残してusage arrayをemptyにする。

以下はproduction rootからjoinできるlogical audit viewであり、各child manifestが
全fieldを重複保持するという意味ではない。resource単位とplacement単位を持つ。

Resource fact:

- SVG SHA-256、byte length、media type
- logical image ID aliasごとのimage ID、declared URI/expected hash、producer
  conversion engine ID/version/rules version（`svg-safe-2`ではrequired、
  `svg-safe-1`ではabsent）
- Typaxis safe-SVG admission attestation fingerprint、parser ID、IR ID/fingerprint、
  allocation charge
- intrinsic viewport、Form content key、selected時だけのobject/resource name
- logical image ID aliases
- total placement countとalias別placement count

Placement fact（kindごとのinapplicable memberはSchema conditionalで禁止する）:

- NodeId、`figure|inline_vector|math_vector|vector_figure|math_vector_block` kind、
  source TeX slice hash（mathだけ）、alt hash、mathのresolved actual hash、
  `inline_vector`のnullable authored actual hash、language
- inlineとmath blockでは`advance`、`ascent`、`descent`、`origin_x`、`baseline`、
  viewport、existing `figure`と`vector_figure`ではselected viewport
- spacingまたはblock style/number owner
- `math_vector_block`だけはmath-vector flow algorithm/fingerprint、`MathVectorFlowId`、
  parent FlowId/position、terminal `1`
- page/frame/fragment/paint ordinal
- viewport rectangle、uniform scale、placement matrix
- SafeVector `/2`のDisplay/PDF-use fingerprint、math-vector `/1`のbinding
  fingerprint、tagged-PDF `/2`のstructure/marked-content fingerprint

配列順はresource factをcontent key、placement factをselected paint orderで
固定する。resource内aliasはnumeric `image_id`、alias内usageはselected paint
orderで固定する。engine versionだけでなく、意味を変え得るparser/IR/layout/
dedupe/rules identityをすべてrecordする。

## 12. Capability descriptor

公開時の`machine_input.profiles`内にある`production-book-1` descriptorへ、
既存fieldを壊さず次を追加する。次のJSONはroot capability objectではなく、
該当profile objectからvector関連memberを抜き出した説明用projectionである。

`blocks`、`inlines.kinds`、`style_block_types`、`style_selectors`、新設の
array-valued `vector_*`はset-valuedなのでUTF-8 byte順にcanonicalizeする。
`vector_features_by_profile`と`vector_media_by_kind`の各value arrayも同じ
UTF-8 byte順にする。JSON object memberはset arrayの規則を流用せず、既存JCS
writerどおりRFC 8785のUTF-16 code-unit lexical順にserializeする。一方、ADR-0036の
resource component/media配列は意味を持つ固定順であり、global sortしない。
`safe-vector/2`を使うproduction resource set `/2`のcomponent順は
PNG、SafeVector、JPEG、TrueType、CFFを保ち、image media順はexact
`png, svg-safe-1, svg-safe-2, jpeg-baseline`、font media順はexisting exact
`sfnt-truetype-glyf, ttc-truetype-glyf, sfnt-cff1`を保つ。

```text
typaxis.resource-profile/png/1
typaxis.resource-profile/safe-vector/2
typaxis.resource-profile/jpeg-baseline/1
typaxis.resource-profile/truetype-glyf/1
typaxis.resource-profile/sfnt-cff1/1
```

```json
{
  "blocks": [
    "math_vector_block",
    "vector_figure"
  ],
  "image_formats": [
    "svg-safe-2"
  ],
  "inlines": {
    "kinds": [
      "inline_vector",
      "math_vector"
    ]
  },
  "style_block_types": [
    "math_vector_block",
    "vector_figure"
  ],
  "style_selectors": [
    "math_vector_block",
    "vector_figure"
  ],
  "vector_features": [
    "clip-path",
    "current-color",
    "paint-opacity",
    "shared-form-xobject"
  ],
  "vector_features_by_profile": {
    "svg-safe-1": [
      "clip-path",
      "shared-form-xobject"
    ],
    "svg-safe-2": [
      "clip-path",
      "current-color",
      "paint-opacity",
      "shared-form-xobject"
    ]
  },
  "vector_formats": [
    "svg"
  ],
  "vector_media_by_kind": {
    "figure": [
      "svg-safe-1"
    ],
    "inline_vector": [
      "svg-safe-1",
      "svg-safe-2"
    ],
    "math_vector": [
      "svg-safe-2"
    ],
    "math_vector_block": [
      "svg-safe-2"
    ],
    "vector_figure": [
      "svg-safe-1",
      "svg-safe-2"
    ]
  },
  "vector_metrics": [
    "advance",
    "ascent",
    "baseline",
    "descent",
    "origin_x",
    "viewport"
  ],
  "vector_profiles": [
    "svg-safe-1",
    "svg-safe-2"
  ]
}
```

既存fieldである`blocks`、`image_formats`、`inlines.kinds`、
`style_block_types`、`style_selectors`について、このprojectionは追加値だけを
示す。実際のprofile descriptorでは既存値とmergeした完全配列を出す。
新設`vector_*` memberは、array-valued memberもobject-valued memberも、示した値が
完全値であり、別の暗黙値とmergeしない。

MI4-13で公開する場合、capability Schemaのprofile tupleは7件から8件へなり、
UTF-8 byte順では`production-book-1`を`paragraph-1`の後、`table-1`の前へ置く。
`default_profile`は引き続き`paragraph-1`である。
descriptorは`typaxis.production-book-resource-set/2`、
`typaxis.resource-profile/safe-vector/2`、上記exact media配列も同時にbindする。

MI4-13より前のpublic `capabilities --format json`はこれらをadvertiseしない。
Schemaだけ、crate-private runnerだけ、またはunit testだけが存在する状態を
`available: true`にしてはならない。

## 13. Error policy

すべてterminal errorであり、対象nodeを省略したPDF successはない。

| condition | phase / code |
| --- | --- |
| missing/unknown/wrong-typed node、metric、source、alternative field | strict decode、`P1102`、exact JSON Pointer |
| empty/control-only `alt` / nonnull `actual_text`、invalid source TextSpan | syntax、`P1102`または既存text-map code |
| metric range/relation/aspect/scale failure | syntax/profile preflight、`P1102`、metric member Pointer |
| duplicate/noncanonical `image_id` declaration | syntax、`P1102`、resource Pointer、resource open前 |
| missing `expected_sha256` memberまたはwrong type | strict decode、`P1102`、member Pointer |
| `svg-safe-2`のnull expected hashまたはmissing/invalid provenance | strict conditional decode、`P1102`、member/resource Pointer |
| profile-disallowed media | profile preflight、`R7100`、resource subject |
| expected/content SHA-256 mismatch | stable resource admission、`R7100` |
| malformed SVG、forbidden element/reference、unsupported feature | Safe-SVG admission、`R7100`、resource subject + typed reason |
| vector node/path/depth limit | existing `R7120` / `R7121` / `R7122` |
| canonical IR allocation、text/AST、selected occurrence、PDF object max+1 | respective `R7111`、`T2100`/`T2101`、`P1120`/`P1121`、`L5110`、`G6100` |
| same declared/admitted SHA-256 with different full stable bytes | cross-resource collision check、`R7100` |
| inline logical/visual oversize、block/page width or height overflow | layout、`L5100`、NodeId/SourceSpan |
| invalid equation-number text/NodeId/Span/minimum gap | syntax、`P1102`または既存text-map code |
| nonpositive equation-number shapeまたはformulaとのcollision | block layout、`L5100` |
| flow/selected placement/Display/Form/PDF/manifest/structure receipt mismatch | internal closure、`I9190` |

`R7100`のtyped reasonは少なくとも`malformed_svg`、`forbidden_feature`、
`external_reference`、`unsupported_feature`、`hash_mismatch`、
`resource_conflict`を区別する。unknown featureをparserがskipした成功状態は
存在しない。

## 14. Crate ownershipと実装箇所

| owner | 変更責務 |
| --- | --- |
| `typaxis-document-package` | new wire kinds、metric/spacing/provenance DTO、strict Schema/JCS |
| `typaxis-document` | typed inline/block/resource domain |
| `typaxis-syntax` | TextSpan/alternative/metric/resource-ref/provenance validation、sealed metric receipt、computed-language registry `/2` |
| `typaxis-style` | `typaxis.precomposed-vector-style/1`、closed property applicability、equation-number text style、既存registry `/1` isolation |
| `typaxis-machine-profile` | accepted kinds/media/metrics/style/language-owner scope、resource-set `/2`、capabilities、pre-resource rejection |
| `typaxis-resource-admission` | `svg-safe-2` stable-byte parser、currentColor/alpha IR、attestation |
| `typaxis-layout-contract` | backend-independent atomic vector item/placement、nominal `MathVectorFlowId` / terminal types |
| `typaxis-linebreak` | synthetic AL unit、`VectorBoundaryItem`、`advance`、max ascent/descent、atomic wrapping |
| `typaxis-layout` | math-vector binding、dense math-vector flow registry、block alignment/number/atomic pagination |
| `typaxis-display-list` | logical DrawVectorResource + semantic marked-content owner |
| `typaxis-resources` | content-key Form dedupe、ExtGState/Form plans |
| `typaxis-pdf` | vector Form serialization、placement `Do`、ActualText、book-navigation/vector PDF observations `/2` |
| `typaxis-manifest` | resource/metric/count/engine/rules/layout/PDF facts、book-navigation manifest `/2` |
| tagged-structure owners | `/2` Formula/Figure/number role、MCR/property-Span、Alt/ActualText/Lang closure |
| `typaxis-testkit` | VMB corpus、limit/tamper mutation、independent renderer/extractor/validator evidence |
| `typaxis-cli` | package check/build integrationとpublic capability gate |

`typaxis-math`のTeX-shaped parserはproducer-composed vectorをparseしない。
external vector metrics専用のpublic constructorを`MathComputationReceipt`へ
追加せず、syntax/layout ownerだけがopaque bindingを発行する。

PDF backend、Display、line breakerはresource URI、raw SVG、raw TeX、caller
safety booleanを受け取らない。各phaseは直前ownerのreceiptだけをconsumeする。

## 15. Acceptance tests

### 15.1 Positive VMB corpus

VMBが生成・`svg-safe-2`へlowerしたgolden resourceをrepositoryへ固定し、
source TeX、speech/alt、metrics、SVG hash、conversion identitiesをfixtureへ
同梱する。最低限、次を含める。

- `x+y`
- `x\sim y`
- `2\nmid 8`
- `(a,b)`
- `\frac{1}{2}=\frac{2}{4}`
- 上付き・下付き
- 大きな括弧
- 行列
- 複数行`aligned`
- page幅近くの長いblock math
- 同じ数式resourceの複数回利用
- 日本語、句読点、括弧に隣接するinline math
- line末候補の直前にあるinline math
- page末候補の直前にあるblock math
- equation number付きblock math
- currentColor、stroke、clip、fill/stroke opacityを各一例
- `alt`とnonnull `actual_text`が異なるmath、およびnull fallbackのmathを各一例
- document language継承と明示overrideを、math/genericかつinline/blockの4 kindで
  少なくとも一例ずつ

### 15.2 Layout assertions

- `viewport_top + baseline == line_baseline_y`を全inline occurrenceで確認する。
- line width factがSVG bboxでなく`advance`を含むことを確認する。
- line頭/末へ移動した数式で該当spacingがzeroになることを確認する。
- opening/closing punctuationと数式の間にpositive spacingを置いても、spacingが
  Unicode/Japaneseのprohibited boundaryをbreakableに変えないことを確認する。
- vector境界でJapanese pair tableのnatural gapが指定spacingへ二重加算されず、
  no-break branchだけがexact gapを持つことを確認する。
- 分数・総和・積分・添字・行列を同一lineに置き、line ascent/descentが最大値、
  next line topが前line bottom以後であることを確認する。
- 数式内部にfragment/break recordがないことを確認する。
- page末でfitしないblockが次pageへ全体移動し、empty pageにもfitしないblockは
  `L5100`になることを確認する。
- `start|center|end`がLTRでleft/center/rightになることとequation numberの独立
  rectangleを確認する。
- new block selectorのcascade/applicabilityが`typaxis.precomposed-vector-style/1`へ
  bindされ、既存basic style registry `/1`へ同じselectorを渡すと拒否されることを
  確認する。
- 番号なしfixtureで`Bh = Vh`となり、number rectangle/paint/structure childが
  存在しないことを確認する。
- 番号がviewportより高いfixtureで`Bh = max(Vh, Nh)`、両childのcenter、paint /
  pagination boundsが一致することを確認する。
- 複数のnative `display_math`と`math_vector_block`を交互に置き、native
  `MathFlowId`とproducer-composed `MathVectorFlowId`が独立してそれぞれdenseに
  なり、各blockがexact terminal `1`を一度だけ消費することを確認する。

### 15.3 PDF assertions

- math paintがForm XObjectのpath operatorであり、image raster subtypeを
  持たないことをindependent parserで確認する。
- Form `/BBox`、clip、stroke width、fill/stroke、ExtGState alpha、placement
  matrixをmanifest期待値と照合する。
- alpha 1のdrawもExtGStateを明示し、先行page/Form paintのalphaやcolorが
  currentColor placementへ漏れないことを確認する。
- 同じSVG hashのN回使用が1 Form object + N `Do`になることを確認する。
- 200%、800%相当または複数render DPIでoutlineがvector sourceから再描画
  され、固定pixel resourceを参照していないことを確認する。
- text extractorの結果がTeX token列でなくexact resolved `actual_text`になる
  こと、および前後の日本語・句読点・番号とのdocument orderが保たれることを
  確認する。
- tagged PDFでmathが`Formula`、generic vectorが`Figure`、Alt/ActualText/Lang、
  equation number reading orderが一致することを確認する。
- computed-language registry `/2`とbook-navigation manifest `/2`が4つのnew kindを
  NodeId順にexactly once含み、selected paint/PDF `/Lang`と同じeffective languageを
  bindすることを確認する。
- Form streamにMCIDがなく、page-level `Do`がouter Formula/Figure MCRとinner
  property-only Spanの正しいnestingにあることを確認する。
- source TeX slice、alt/resolved actual、全metric、engine/rules identity、resource
  placement count、block flow/terminalがmanifestのexact hash/valueと一致することを
  確認する。
- 公開gateでは`typaxis capabilities --format json`の`production-book-1`が12章の
  kind/format/profile/metric/feature、resource-set `/2` identity、component/media順を
  exactにadvertiseし、同じdescriptorがpreflightを駆動することを確認する。
- in-tree tagged validator `/2`、pinned veraPDF、Matterhorn assessment ledger `/2`が
  新しいFormula/Figure/number mappingを同じfixture revisionで閉じることを
  確認する。
- 既存SafeVector/Display/Form/PDF/manifest `/1`、native `typaxis.math-flow/1`、
  computed-language/book-navigation chain `/1`のSchema、canonical record、golden
  bytesが変わらないことを確認する。producer-composed fixtureは3章のnew identity
  だけを参照し、legacy `/1` receiptを代替として受理しない。

### 15.4 Negative tests

- malformed XML/SVG、unknown element/attribute/path command
- `script`、event、animation、CSS、`use`、group/object `opacity`、
  text/font/image、external/data URI
- noncanonical `currentColor`、invalid fill/stroke-opacity lexical/range、
  paint alphaがすべてzeroのresource
- missing/wrong hash、same ID/different declaration、malformed/missing provenance、
  same declared hash/different full bytes
- missing metric、zero/negative/overflow、baseline外、ascent/descent不足
- intrinsic ratio不一致、nonuniform scale、page/frame width超過
- empty/control-only alt/actual text、invalid TextSpan
- invalid/duplicate equation-number owner、invalid gap、zero shape、formula collision
- missing/wrong style receipt、new selectorと既存basic style registry `/1`のswap
- missing/duplicate/non-dense math-vector flow、wrong parent/position/terminal、native
  `MathFlowId`または`typaxis.math-flow/1` receiptとのnamespace swap
- content-key/Form/use/manifest/structure tamper
- missing/extra/wrong-kind language owner、`/1`と`/2`のlanguage/navigation receipt swap
- exact vector node/path/depth limitとmax+1
- exact/max+1のIR allocation、text、AST、selected occurrence、PDF object limit
- old profile rejectionとpublic capability isolation

### 15.5 Reproducibility

同じfixtureと同じlogical IDsを異なるcheckout pathからbuildし、PDFと全
sidecarをbyte比較する。cross-ID dedupeは一つのpackage内で同じSVG hashを
異なるprovenanceを持つ二つのresource IDから参照し、alias provenanceは二件
残る一方でForm objectだけが共有されることを別testで確認する。

## 16. 実装順

以下はtechnical dependency順であり、現行
[`docs/25` task plan](25-machine-input-pdf-improvements-todo.md)のmilestoneへは
まだ割り当てられていない。現行MI4-11はJPEG、MI4-12はCFF、MI4-13はatomic
publicationだけに閉じているため、この機能をそれらへ暗黙追加してはならない。
採用decision-gateは、MI4-13より前に必要なprivate implementation/evidence
milestoneとdependencyをdocs/25へ明示追加してからstep 1を開始する。既存
MI4-13までにその順序を確保できない場合は、1章どおり公開済み1.4へ後付けせず
新contract/profileへ送る。

1. VMBの代表`texToSvg` corpusと`svg-safe-2` lowering outputを固定し、VMBが
   `use`等を展開してrequired hash/metrics/provenanceを出せることをinterface
   gateで確認する。
2. decision-gate ADRで1.4 private targetへの追加、wire kind、metric invariants、
   `svg-safe-2`、SafeVector/resource-set `/2`、Display/Form/PDF/manifest identity、
   precomposed-vector style `/1`、math-vector flow `/1`、
   computed-language/book-navigation `/2`、
   tagged-structure `/2`、
   diagnostic/capability identityを固定する。
3. resource admissionへ`svg-safe-2`とcurrentColor/paint opacityを追加し、既存
   `svg-safe-1` goldensがbyte-frozenであることを確認する。
4. Form finalizationをcontent-key dedupeへ変更し、Figure経路のregressionを
   閉じる。
5. wire/domain/syntax/profileへ`inline_vector` / `math_vector`とsealed metrics
   bindingを追加する。
6. canonical paragraph itemizationへatomic vector、conditional spacing、dynamic
   line ascent/descentを統合する。
7. `vector_figure` / `math_vector_block`、nominal `MathVectorFlowId`、alignment、
   number、atomic paginationを統合する。
8. Display/PDF/manifest/tagged structureを同じversioned receipt chainへ閉じる。
9. VMB combined fixture、negative corpus、independent PDF/extraction/accessibility、
   deterministic two-build gateを完了する。
10. MI4-13のcomplete production profileでのみpublic capabilityを有効にする。

各stepはcrate-private staging testから始める。partial wire、parserだけ、
layoutだけをpublic profileへadvertiseしない。

## 17. 採用しない代替案

### 17.1 TypaxisでTeXを再組版する

VMBの`texToSvg`と別のgrammar/font/MATH engineを使うとvisual差が生じ、matrix、
aligned、package macro等を再実装する必要がある。今回のsource of truthは
producer-composed vectorなので採用しない。

### 17.2 SVGをPNGへ変換する

拡大品質、path identity、Form reuseを失い、要件を満たさない。

### 17.3 一般SVGをbrowser engineで描画する

CSS、font、external resource、script、filter、platform差、tool versionを
trust boundaryへ持ち込む。closed Safe-SVG subsetでfail closedにする。

### 17.4 1-page PDF fragmentをそのままimportする

PDF pageはcontent streamだけでなく、resource dictionary、font、image、
ExtGState、pattern、annotation、action、embedded file、incremental update、
object stream、filter等を持ち得る。VMBの検査結果booleanを信頼するだけでは
Typaxis自身の安全性・determinism・object closureを証明できない。

将来必要なら、whole PDFではなく、closed operator/resource subsetを持つ
`pdf-form-safe-1`と独立validatorを新しいmedia/profileとして設計する。
初版はすでに存在するSafe-SVG pathを使う。

## 18. 要求トレーサビリティ

| 要求 | 設計箇所 |
| --- | --- |
| 1 inline SVG | 4.1〜4.3、5、6 |
| 2 block SVG | 4.4、7 |
| 3 vector PDF embedding | 8、9 |
| 4 vector PDF fragment alternative | 2、17.4で初版非採用理由と将来境界を明示 |
| 5 math metrics | 4.2、5 |
| 6 inline line breaking | 6 |
| 7 accessibility | 10 |
| 8 resource dedupe | 9.2 |
| 9 deterministic output/manifest | 11 |
| 10 capabilities | 12 |
| 11 error handling | 13 |
| 12 acceptance tests | 15 |
