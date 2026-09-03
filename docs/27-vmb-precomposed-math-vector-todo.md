# VMB向け組版済み数式ベクター配置 実装タスク

Source: `docs/27-vmb-precomposed-math-vector.md`

- Design baseline commit: `7d9a03ca9f34fa2bb659c3ac92f9f028d57ac7ff`（以後のreview fixは設計書と本書を同一change setで更新する）
- 状態: Pending
- 対象: 非公開の`typaxis.contract/1.4` / `typaxis.machine-pdf/production-book-1`
- 公開owner: `MI4-13`
- 前提: `MI4-02`、`MI4-04`、`MI4-05`、`MI4-07`、`MI4-09`、`MI4-10`がCompletedである現行repository

この文書は、設計書で未割当だったproducer-composed vector機能を、既存M4 taskと衝突しない`MI4-V01`〜`MI4-V19`へ分解する。`MI4-V01`〜`MI4-V18`はcrate-privateな1.4 stagingだけを実装し、public current contract 1.3、七profile、default `paragraph-1`、公開Schema alias、CLI help、capability bytesを変更しない。`MI4-V19`は公開可能性を証明するfeature-local gateであり、public aliasと`production-book-1`を有効化する唯一のownerは、引き続きmaster planの`MI4-13`である。

本拡張の詳細task/acceptanceは本書をsingle sourceとし、authoritative release schedulingは`docs/25-machine-input-pdf-improvements-todo.md`である。`MI4-V02`は[ADR-0037](../adr/ADR-0037-producer-composed-math-vector.md)と短いmilestone stub/dependency/linkをmasterへ登録済みであり、masterがrelease status/dependency、本書が詳細task/acceptanceを所有する。

この文書の`Completed`は各milestoneの受け入れ条件を満たしたことだけを意味する。Wire DTO、Schema、parser、layout、PDF、manifestの一部が存在しても、`MI4-V18`のcombined gateと`MI4-V19`のpublication-readiness gateが完了するまではこの機能を利用可能と表明しない。

## 1. Scope

### 1.1 実装するもの

- VMBがstable resourceとして渡す`svg-safe-2`、required SHA-256、conversion provenance
- `inline_vector`、`math_vector`、`vector_figure`、`math_vector_block`
- `pdf_point_1_65536`固定小数点のproducer metrics、baseline、advance、spacing
- inline atomic itemization、Unicode/Japanese boundary、dynamic line ascent/descent
- block alignment、equation numberの独立配置、atomic pagination、overflow error
- Safe-SVG 2の`currentColor`、`fill-opacity`、`stroke-opacity`
- canonical vector IRからのPDF Form XObject、ExtGState、content-key dedupe
- opaque source TeX、alternative、resolved ActualText、computed language、Formula/Figure構造
- SafeVector `/2`、math-vector `/1`、book-navigation `/2`、tagged-PDF `/2`のmanifest closure
- private capability projection、VMB combined corpus、negative/tamper、determinism、独立PDF検査

### 1.2 実装しないもの

- TypaxisによるTeX parse、macro expansion、数式組版、読み上げ文生成
- native `inline_math` / `display_math`からvector kindへの暗黙変換、または逆方向fallback
- SVGのrasterization、browser engine、CSS、font、text、image、network/file/data URI
- SVG内部のline、fragment、page分割、自動縮小、crop、page回転
- 一般的な1-page PDF fragment import。将来必要な場合は`pdf-form-safe-1`相当を別設計する
- decorative inline vector、Typaxisによるequation number生成・increment・localize
- current 1.3、既存profile、既存`/1` receiptの意味変更

### 1.3 開始時のcontract分岐

`MI4-V02`開始時に`MI4-13`の実状態を確認する。

- `MI4-13`が未完了かつ1.4が未公開なら、この文書どおりprivate 1.4へ追加し、master planの`MI4-13`を`MI4-V19`依存へ更新する。
- `MI4-13`がCompletedまたは1.4が公開済みなら、1.4へ後付けしない。新contract/profileを採番するADRを作り、この文書のcontract、Schema、fixture、publication dependencyを更新してquality gateを再実行するまで`MI4-V03`以降を開始しない。

MI4-V02は2026-09-03にcurrent 1.3、public seven-profile、private 1.4、
`MI4-13 = Pending`を確認し、前者の分岐をADR-0037として採用した。以後も
MI4-V03開始時にMI4-13が先行公開されていないことを再確認し、公開済みなら後者へ
切り替える。

## 2. 全milestone共通の実装規則

### 2.1 Trust boundaryとphase ownership

- Wire object、resource URI、expected hash、provenance、metrics、TeX、alternativeはuntrustedである。
- strict decodeとsyntax validationを通した後も、resource-backed factはstable-byte admissionが発行したattestationと一致するまでtrustedにしない。
- metric、math binding、style、profile、selected layout、Display、Form plan、PDF、language、structure、manifestのreceiptはpublic raw-parts constructor、public mutable field、caller supplied fingerprintを持たない。
- PDF backend、Display、line breakerはresource URI、raw SVG、raw TeX、caller supplied safety booleanを受け取らない。直前ownerのsealed receiptだけをconsumeする。
- `expected_sha256`はadmitted full stable bytesから再計算したSHA-256と照合し、未照合の文字列をcontent keyへ使わない。
- unsupported node/media/styleはresource open前のprofile preflightで拒否する。malformed SVG等のresource-local errorはstable read後、layout開始前に拒否する。
- error時に対象nodeを省略したPDF success、PNG fallback、native math fallback、warning-only omissionを作らない。

### 2.2 Versioned identityと旧経路の凍結

新経路は次のidentityをexactに使う。ADRで変更した場合は、この文書、Schema、capability、fixtureを実装開始前に同時更新する。

| owner | identity |
| --- | --- |
| wire media | `svg-safe-2` |
| SafeVector component | `typaxis.resource-profile/safe-vector/2` |
| production resource set | `typaxis.production-book-resource-set/2` |
| parser / IR / IR fingerprint / allocation | `typaxis.safe-svg-parser/2`、`typaxis.safe-vector-ir/2`、`typaxis.safe-vector-ir-fingerprint/2`、`typaxis.safe-vector-allocation-charge/2` |
| metrics / style / math binding | `typaxis.precomposed-vector-metrics/1`、`typaxis.precomposed-vector-style/1`、`typaxis.precomposed-math-binding/1` |
| inline / block layout | `typaxis.atomic-vector-inline/1`、`typaxis.math-vector-flow/1`、`typaxis.precomposed-vector-layout/1` |
| Display / dedupe | `typaxis.draw-vector-display/2`、`typaxis.vector-form-dedupe/1` |
| Form plan / PDF | `typaxis.safe-vector-form-plan/2`、`typaxis.safe-vector-form-plans/2`、`typaxis.safe-vector-pdf-closure/2` |
| vector manifests | `typaxis.safe-vector-manifest/2`、`typaxis.math-vector-manifest/1` |
| language/navigation | `typaxis.computed-language-registry/2`、`typaxis.book-navigation-profile-view/2`、`typaxis.book-navigation-profile-receipt/2`、`typaxis.book-navigation-selected/2`、`typaxis.book-navigation-pdf/2`、`typaxis.book-navigation-manifest/2` |
| accessibility | `typaxis.pdfua1-profile/2`、`typaxis.production-accessibility-preflight/2`、`typaxis.production-accessibility-authorization/2`、`typaxis.structure-role-vocabulary/2`、`typaxis.structure-registry/2`、`typaxis.selected-structure-binding/2`、`typaxis.marked-content-plan/2`、`typaxis.tagged-pdf-observation/2`、`typaxis.tagged-pdf-validator/2`、`typaxis.tagged-pdf-manifest/2`、`typaxis.pdfua1-validation-policy/2`、`typaxis.matterhorn-assessment/2` |

既存`svg-safe-1` parser/IR、`typaxis.basic-block-style-registry/1`、`typaxis.basic-flow-registry/1`、`typaxis.semantic-container-flow-registry/1`、`typaxis.math-flow/1`、native `MathFlowId`のcanonical JCS、fingerprint、Schema、golden bytesを変更しない。SafeVectorでは`typaxis.safe-vector-selected-layout/1`、`typaxis.draw-vector-display/1`、`typaxis.safe-vector-form-plan/1`、`typaxis.safe-vector-form-plans/1`、`typaxis.safe-vector-pdf-closure/1`、`typaxis.safe-vector-manifest/1`を凍結する。native mathの`typaxis.math-manifest/1`、language/navigationの`typaxis.computed-language-registry/1`、`typaxis.book-navigation-profile-view/1`、`typaxis.book-navigation-profile-receipt/1`、`typaxis.book-navigation-selected/1`、`typaxis.book-navigation-pdf/1`、`typaxis.book-navigation-manifest/1`、accessibilityの`typaxis.tagged-pdf-manifest/1`も凍結する。`svg-safe-1`を`/2` parserで再解釈せず、`svg-safe-2`を`/1` parserへfallbackしない。

### 2.3 Wire kind、media、semantic role

| kind | placement | allowed media | source TeX | PDF role | ActualText rule |
| --- | --- | --- | --- | --- | --- |
| `inline_vector` | inline atomic | `svg-safe-1`または`svg-safe-2` | forbidden | Figure | nonnull authored値だけを使用。nullからAltを生成しない |
| `math_vector` | inline atomic | `svg-safe-2`だけ | required | Formula | nonnull authored値、nullならAltへexact fallback |
| `vector_figure` | block atomic + caption flow | `svg-safe-1`または`svg-safe-2` | forbidden | Figure | paint-level ActualTextなし |
| `math_vector_block` | block atomic + one-terminal flow | `svg-safe-2`だけ | required | Formula | nonnull authored値、nullならAltへexact fallback |

既存`figure`のvector branchは`svg-safe-1`だけ、既存`inline_math` / `display_math`はnative math経路だけという意味を維持する。

### 2.4 Metric、scale、baseline

`PrecomposedVectorMetrics`は`advance`、`ascent`、`descent`、`origin_x`、`baseline`、`viewport.width`、`viewport.height`を持つ。全値の単位はroot `coordinate_unit = pdf_point_1_65536`だけで宣言し、JSON numberはcanonical safe integerとする。`origin_x`だけsigned Length、`descent`とspacingはnonnegative、その他はpositiveまたは下記関係を満たすLengthとする。

```text
advance > 0
ascent > 0
descent >= 0
viewport.width > 0
viewport.height > 0
0 <= baseline <= viewport.height
ascent >= baseline
descent >= viewport.height - baseline
```

admitted intrinsic sizeを`Iw` / `Ih`、node viewportを`Vw` / `Vh`とし、checked `i128`とround-half-to-evenで一つの16.16 scaleを導出する。

```text
s = round_half_even(Vw * 65536 / Iw)
scale(Iw, s) == Vw
scale(Ih, s) == Vh
```

`s`は`positive_unitless_16_16`へ収まり、`origin_x + Vw`はchecked計算できなければならない。x/y別scale、float、ambient font size、SVG unit suffixからのnode metric再計算を禁止する。

inline placementは次を唯一のbaseline式とする。

```text
viewport_left = pen_x + origin_x
viewport_top = line_baseline_y - baseline
line_baseline_y = viewport_top + baseline
```

line widthは`advance`、visual frame fitは`origin_x .. origin_x + viewport.width`、line heightは全text/vectorのmaximum ascent/descentを使う。block alignment/overflowはviewport widthを使い、`math_vector_block.metrics.advance`はbinding/manifestだけに保持する。

### 2.5 Style、block、pagination

- `math_vector_block`と`vector_figure`は`typaxis.precomposed-vector-style/1`だけでcascadeする。
- 両kindに`space_before`、`space_after`、`start_indent`、`end_indent`、`text_align`、`page`、`keep_with_next`を適用する。
- `vector_figure`だけに`keep_caption`を適用する。
- `math_vector_block`の`font_family`、`font_size`、`line_height`はequation-number textだけに適用し、SVG metrics/scaleを変えない。
- `width`は両kindでinapplicable、`keep_caption`は`math_vector_block`でinapplicableとし、`L5101`にする。
- `MathVectorFlowId`はnative `MathFlowId`とnominalにも採番空間にも別で、validated documentの`math_vector_block` NodeId preorderから0始まりdenseに発行する。各flow terminalはexact `1`である。
- nonnull `equation_number`は`math_vector_block`唯一のsource-owned leaf childで、owner直後のglobal dense typed-preorder NodeId、depth `owner + 1`を使う。nullはchild/NodeIdを消費しない。formula/number TextSpanを覆うidentity TextMapの対応SourceSpanはownerと同じ`source_id`で包含・非重複とし、`formula_source_span.end_byte <= equation_number.span.start_byte`を要求する。
- 4 kindはいずれもsemantic-containerのauthored-content判定でnonemptyである。required meaningful `alt`とatomic vector paintを根拠とし、path数、TeX、caption、番号の有無で空判定しない。
- blockはviewportまたは`Bh = max(Vh, Nh)`のatomic rectangleとして扱い、page/frameへfitしなければ全体を次frameへ送る。empty full frameにもfitしなければ`L5100`にする。
- overflow policyは常に`error`。shrink、crop、分割、番号とのcollision回避、keepの暗黙解除をしない。

### 2.6 Safe-SVG 2、paint、dedupe

- Safe-SVG 2はSafe-SVG 1のclosed subsetへexact `currentColor`、presentation attributeの`fill-opacity` / `stroke-opacity`だけを追加する。
- paint IRは`None | FixedRgb8 | CurrentColor`とresolved scalar alphaを持つ。`style`、CSS、`color`、group/object `opacity`、mask、filter、blend、font/text/image、`use`、external referenceを拒否する。
- opacity初期値はexact 1、child specified値はinherit値を置換し、親子で乗算しない。enabled fill/strokeのpositive alphaが一つもないresourceを拒否する。
- Formはalpha pairごとのExtGStateを値の昇順で持ち、`(1, 1)`も明示する。CurrentColorはplacementのresolved text paintをstroking/nonstroking両方へ設定してから`Do`し、`q`/`Q`で隔離する。
- Form dedupe keyはtuple `VectorContentKey(source_sha256, media_type, parser_id, ir_id, ir_fingerprint)`。曖昧な文字列連結、first-use順、resource ID、page、NodeId、provenance、resolved colorをkeyに含めない。
- Form planのrelative object-role orderとresource nameは`VectorContentKey`のcomponent-wise lexical order、Form-local ExtGState nameはalpha pair orderで割り当てる。absolute PDF object numberはcomplete final graph ownerだけが割り当てる。
- 同一keyの複数resource IDは一つのFormを共有するが、alias別provenance、usage、placement countを失わない。zero-use resourceはfactだけを残し、Form objectを作らない。

### 2.7 Alternative、language、tagged PDF

- `alt`はrequiredで、Unicode 16.0 `White_Space`以外を少なくとも一scalar含み、C0/C1 controlを含まない。trim、normalize、collapseしない。
- mathの`source_tex.text_span`はnonempty UTF-8、BOM/NULなしのexact sliceをidentity TextMapで参照し、parse/normalizeしない。
- `language`は読み上げ文のBCP 47 overrideであり、既存`typaxis.bcp47-language/1`でcanonicalizeする。4 kindを含むowner registryだけを`/2`へする。
- Formula/Figure構造のouter MCRがMCIDを所有し、必要な`ActualText` / paint-level `Lang`はMCIDを持たないinner property-only Spanで`Do`だけを囲む。再利用Form streamへMCID、Alt、ActualText、Langを入れない。
- equation numberはFormula vector MCRに続くsource-owned Span childであり、親のcomputed languageを使う。number textをformula ActualTextへ重複合成しない。
- PDFへopaque TeXの独自dictionary keyやattachmentを追加しない。TeXはTextStoreとmanifestのspan/hash closureで保持する。

### 2.8 Limits、diagnostics、failure side effects

- SVG bytesは`max_image_bytes` / `max_resource_bytes`、vector node/path/depthは`max_vector_nodes` / `max_vector_path_segments` / `max_vector_nesting_depth`へone-time chargeし、それぞれ`R7120` / `R7121` / `R7122`で拒否する。IR allocationは`max_decoded_image_bytes`へ課金し、max+1を`R7111`で拒否する。
- Safe-SVG 2 allocation chargeはchecked `64 * nodes + 80 * stored_segments + 48 * paint_or_clip_commands + source_clip_id_bytes`である。
- TeX、alt、nonnull actual、language、equation-number textは`max_text_buffer_bytes` / `max_text_bytes`へexactly once課金する。math null fallbackはAltのaliasで再課金しない。
- semantic vector/equation-number nodeは`max_ast_nodes` / `max_ast_nesting_depth`、selected vector occurrenceは`max_fragments`へ課金する。vector planning ownerはdedupe後のForm/ExtGState等のrelative object-role count deltaをchecked計算するだけでglobal object budgetをconsumeしない。complete final indirect-object graph ownerがvector以外を含む全actual objectへabsolute numberを割り当てる直前に一回だけ`max_pdf_objects`をconsumeする。Form plan/page spoolは`max_spool_bytes`、final writeは`max_output_bytes`を既存ownerでconsumeする。
- duplicate ID、missing field、invalid metric/text/spanは`P1102`、profile media mismatchは`R7100`、SVG admissionはtyped reason付き`R7100`、vector limitsは`R7120`〜`R7122`、layout overflow/collisionは`L5100`、style applicabilityは`L5101`、selected countは`L5110`、PDF objectは`G6100`、receipt tamperは`I9190`とする。
- `R7100` reasonは少なくとも`malformed_svg`、`forbidden_feature`、`external_reference`、`unsupported_feature`、`hash_mismatch`、`resource_conflict`を区別する。
- terminal failureは既存atomic publication順に従い、partial PDF success、対象elementの欠落、空の合成receiptを出さない。

### 2.9 Determinism、fixture、evidence

- canonical collectionは`BTreeMap`/`BTreeSet`または明示sortを使い、HashMap insertion、filesystem order、worker completion、first page useをartifact順へ使わない。
- resource factはcontent key順、aliasはnumeric image ID順、usage/placementはselected paint order、math-vector flowはsource preorderで固定する。
- byte-for-byte determinism比較は同一package/resource bytes、dense IDs、selected paint orderを使い、owner-private candidate列挙順またはworker completion scheduleだけを変える。wire resource declaration順は入力契約の一部なので、入替えをsame-input testに使わない。
- generated PDF/sidecar/evidenceはversioned sample directoryへ書かず、test temporary directoryまたは`target/machine-e2e/`へ書く。
- VMB positive corpusはversioned inputとしてcheck inし、生成物の再生成scriptがある場合もexpected SVG/hash/metricsを実行時に暗黙更新しない。
- independent evidenceは既存のin-tree PDF parser、MuPDF/Poppler、pinned veraPDF、Matterhorn ledgerを使う。GitHub Actionsや`.github/workflows/`を作成・利用しない。

### 2.10 Capability staging contract

private production descriptorは既存値へ次のaddition/complete vector valueを重ねる。set-valued arrayは記載値をUTF-8 byte順にcanonicalizeし、object keyはJCS順にする。

- block addition: `math_vector_block`、`vector_figure`
- inline kind addition: `inline_vector`、`math_vector`
- style block/selector addition: `math_vector_block`、`vector_figure`
- complete coarse `image_formats`: `jpeg`、`png`、`svg`。このfieldへ`svg-safe-1|svg-safe-2`を入れない
- complete `vector_formats`: `svg`
- complete `vector_profiles`: `svg-safe-1`、`svg-safe-2`
- complete `vector_metrics`: `advance`、`ascent`、`baseline`、`descent`、`origin_x`、`viewport`
- complete `vector_features`: `clip-path`、`current-color`、`paint-opacity`、`shared-form-xobject`
- `svg-safe-1` features: `clip-path`、`shared-form-xobject`
- `svg-safe-2` features: `clip-path`、`current-color`、`paint-opacity`、`shared-form-xobject`
- kind/media mapping: existing `figure -> svg-safe-1`、`inline_vector -> svg-safe-1|svg-safe-2`、`math_vector -> svg-safe-2`、`math_vector_block -> svg-safe-2`、`vector_figure -> svg-safe-1|svg-safe-2`

production resource component順は`typaxis.resource-profile/png/1`、`typaxis.resource-profile/safe-vector/2`、`typaxis.resource-profile/jpeg-baseline/1`、`typaxis.resource-profile/truetype-glyf/1`、`typaxis.resource-profile/sfnt-cff1/1`とする。image media順はexact `png, svg-safe-1, svg-safe-2, jpeg-baseline`、font media順はexact `sfnt-truetype-glyf, ttc-truetype-glyf, sfnt-cff1`とする。公開時のprofile tupleは8件で、`production-book-1`を`paragraph-1`の後かつ`table-1`の前へ置き、defaultは`paragraph-1`のままとする。`MI4-13`より前はこのprivate projectionをpublic serializerへ接続しない。

### 2.11 Milestone completion protocol

各milestoneの実装者は次を行う。

1. `Depends on`の全milestoneがCompletedであることを確認する。
2. listed primary files以外へ変更が必要なら、責務境界とdependencyを再確認し、この文書を先に更新する。
3. milestone固有のtargeted verificationを実行する。
4. `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`と変更crateの全testを実行する。
5. closed enum/recordへvariantを追加するmilestoneは、workspace内の全exhaustive consumerを同じchange setで更新する。後続milestoneが機能を有効化するまではtyped terminal staging errorを返すexplicit armを置き、silent ignore、placeholder receipt、既存kindへのfallbackを使わない。`cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked`でincremental compile boundaryを閉じる。
6. Wire、Schema、profile、artifactを変えるmilestoneはpositive、invalid、old-profile rejection、canonical round-tripを同じchange setへ含める。Schema pathをPrimary filesへ持つmilestoneでは`schemas/validate.py`もshared primary fileとし、新しいSchemaを追加・renameする場合は`schemas/README.md`も同じchange setで更新する。
7. public/current isolation assertionを`MI4-V19`まで維持する。
8. statusをCompletedへ変える前に、受け入れ条件をobservable evidenceで確認し、implementation commitと実行環境・command結果を追記する。

## 3. Dependency map

```text
MI4-02 + MI4-04 + MI4-05 + MI4-07 + MI4-09 + MI4-10 -> MI4-V01
MI4-V01 -> MI4-V02
MI4-V02 -> MI4-V03
MI4-V03 -> MI4-V04
MI4-V03 -> MI4-V06
MI4-V04 -> MI4-V05
MI4-V06 -> MI4-V07
MI4-V04 + MI4-V05 + MI4-V06 + MI4-V07 -> MI4-V08
MI4-V08 -> MI4-V09
MI4-V08 -> MI4-V10 -> MI4-V11
MI4-V07 + MI4-V09 + MI4-V11 -> MI4-V12
MI4-V07 + MI4-V12 -> MI4-V13
MI4-V04 + MI4-V09 + MI4-V11 -> MI4-V14
MI4-V12 + MI4-V14 + MI4-09 -> MI4-V15
MI4-V13 + MI4-V15 -> MI4-V16
MI4-V07 + MI4-V13 + MI4-V14 + MI4-V16 -> MI4-V17
MI4-V17 -> MI4-V18
MI4-V18 + MI4-11 + MI4-12 -> MI4-V19
MI4-V19 -> MI4-13
```

`MI4-V04`（syntax metrics）と`MI4-V06`（Safe-SVG 2）は`MI4-V03`後に並行実装できる。`MI4-V05`のprofile authorizationはV04のvalidated packageを入力にする。inlineとblock modelは`MI4-V08`後に並行できる。PDF、language/navigation、tagged structure、manifestはreceipt dependency順に直列化する。

## 4. Milestone summary

| ID | outcome | public surface |
| --- | --- | --- |
| MI4-V01 | VMB interface corpusを固定 | 変更なし |
| MI4-V02 | 採用ADRとmaster dependencyを確定 | 変更なし |
| MI4-V03 | Wire / Schema / domainを追加 | private 1.4だけ |
| MI4-V04 | metric/source/alternative validationを追加 | private receiptだけ |
| MI4-V05 | vector styleとprofile authorizationを追加 | private descriptorだけ |
| MI4-V06 | Safe-SVG 2 admissionを追加 | private resource branchだけ |
| MI4-V07 | content-key/dedupe planning primitiveを追加 | private candidateだけ |
| MI4-V08 | source/vector/metric bindingとlayout contractを閉じる | private receiptだけ |
| MI4-V09 | inline itemization/line breakingを実装 | private layoutだけ |
| MI4-V10 | block flowとequation-number shapeを実装 | private flowだけ |
| MI4-V11 | block placement/paginationを実装 | private layoutだけ |
| MI4-V12 | Display `/2`を実装 | private display artifactだけ |
| MI4-V13 | vector PDF/Form/ExtGState closureを実装 | private PDFだけ |
| MI4-V14 | computed language/book navigation `/2`を実装 | private artifactsだけ |
| MI4-V15 | structure/marked-content `/2`を実装 | private planだけ |
| MI4-V16 | tagged PDF/validator `/2`を実装 | private evidenceだけ |
| MI4-V17 | vector/math/build manifestとcapability stagingを閉じる | public advertiseなし |
| MI4-V18 | VMB combined/negative/determinism gateを閉じる | crate-private runnerだけ |
| MI4-V19 | 外部検査とpublication readinessを証明 | MI4-13へhandoff |

次のPrimary fileは現時点では存在せず、該当milestoneが新規作成する。それ以外のlisted pathは既存ownerを変更する。

- MI4-V01: `samples/machine-package/staging/production-book-1/precomposed-vector/`
- MI4-V17: `workspace/crates/typaxis-manifest/src/math_vector.rs`、`schemas/1.4/machine-math-vector-manifest.schema.json`
- MI4-V18: `schemas/1.4/machine-precomposed-vector-evidence.schema.json`、`tools/verify_precomposed_vector.py`、`tools/test_precomposed_vector.py`

## 5. Detailed milestones

### MI4-V01 VMB interface corpusとlowering boundaryを固定する

- Status: Completed
- Depends on: MI4-02, MI4-04, MI4-05, MI4-07, MI4-09, MI4-10
- Design inputs: docs/27 §2、§4、§8、§15.1、§16 step 1
- Primary files:
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
  - `docs/27-vmb-precomposed-math-vector-todo.md`
- Deliverables:
  - VMBからTypaxisへ渡すchecked-in Safe-SVG 2 positive corpus。
  - source TeX、alt、actual text、language、fixed-point metrics、SVG SHA-256、engine/rules identityを結ぶcanonical corpus ledger。
  - VMB lowering責務とTypaxis受理責務のinterface gate。
- Tasks:
  1. `x+y`、`x\sim y`、`2\nmid 8`、`(a,b)`、`\frac{1}{2}=\frac{2}{4}`、`\sum_{i=1}^{n} i`、`\int_0^1 x\,dx`、上付き/下付き、大括弧、行列、複数行`aligned`、長いblock、equation number、currentColor、stroke、clip、fill/stroke opacityを個別caseとしてcheck inする。
  2. 各caseへexact source TeX bytes、alt、nullable actual text、language、`advance/ascent/descent/origin_x/baseline/viewport`、expected SHA-256、`engine_id`、`engine_version`、`rules_version`、intended kindを記録する。
  3. `defs/path/use`等をVMB側で展開し、SVGがSafe-SVG 2 subsetだけを使うことを確認する。Typaxis側にVMB固有preprocessorを追加しない。
  4. 同一SVG bytesを異なるresource ID/provenanceから参照するdedupe case、同一resourceを10回参照するcaseを用意する。
  5. 日本語、句読点、opening/closing bracket、line末候補、page末候補、異なる高さの複数mathを含むdocument fragmentsをledgerへ結ぶ。
  6. corpus ledgerのpath containment、UTF-8、hash、canonical integer、metric basic relation、duplicate case/resource IDをtestkitで検査する。testはSVGやexpected hashを再生成・更新しない。
  7. VMBがrequired hash/metrics/provenanceを生成できないcaseを列挙し、producer修正が必要なら本milestoneをPendingのまま止める。Typaxis fallbackをinterface解決策にしない。
- Acceptance criteria:
  - 設計§15.1の全positive categoryがcase IDへ一意に対応する。
  - 全SVGのrecorded SHA-256がfull bytesと一致し、metricは整数だけである。
  - math用caseはすべて`svg-safe-2`へlower済みで、`use`、CSS、font、text、external referenceを含まない。
  - same-content aliasと10-use caseがForm dedupe検査に十分な別ID/placement情報を持つ。
  - corpusの生成元tool identityとrules identityが空でなく128-byte printable ASCII上限内である。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit vmb_precomposed_vector_corpus --locked`
  - `rg -n 'x\\+y|x\\\\sim y|2\\\\nmid 8|frac|sum|int|aligned|currentColor|fill-opacity|stroke-opacity' samples/machine-package/staging/production-book-1/precomposed-vector`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V01 change set containing this completion record.
  - `resources.tsv`、`cases.tsv`、`fragments.tsv`をcanonical producer-interface ledgerとして追加し、13 logical resources、18 semantic cases、8 ordered document fragmentsを結んだ。全12 unique SVGはVMB側でpath/shapeへlower済みのSafe-SVG 2 positive bytesであり、raw `use`、CSS、font/text、script/animation、embedded/external referenceを含まない。
  - source TeX、alt/nullable actual text、inherit/override language、`pdf_point_1_65536` metric/spacing、full SVG SHA-256、engine/version/rules identity、intended kindを固定した。同一bytesの2 logical ID/異なるprovenance、同一caseの10 placement、Japanese boundary、line/page末候補のfixed fit context、mixed-height、numbered blockをfragment occurrence ordinalへ結んだ。
  - `typaxis-testkit`のcorpus-only gateはcontained regular path、canonical UTF-8/LF/TSV/integer、dense unique ID、full-byte hash、printable provenance、metric relationとone-scale viewport、kind別field、exact TeX/category/language binding、Safe-SVG 2 positive subset、alias/10-use/fragment coverageを検証する。欠落metric、非canonical integer、hash mismatch、duplicate ID、forbidden `use`/external image、invalid alpha等のmutantも拒否する。
  - milestone指定のtargeted test/`rg`、testkit全test、testkit all-target clippy `-D warnings`、workspace fmt check、`python3 schemas/validate.py`、`/usr/bin/git diff --check`はいずれもexit 0。evidenceは`workspace/crates/typaxis-testkit/src/lib.rs`と`samples/machine-package/staging/production-book-1/precomposed-vector/`にある。
  - Typaxis product code、public command/profile、contract、Schema、capabilityは変更していない。MI4-V02採用前のpre-adoption evidenceに閉じており、scope deviationおよび新規採択ADRはない。
- Non-goals:
  - Typaxis product code、Schema、capabilityの変更
  - VMBのTeX engine自体の再実装

### MI4-V02 採用ADRとM4 publication dependencyを確定する

- Status: Completed
- Depends on: MI4-V01
- Design inputs: docs/27 §1、§3、§12、§16
- Primary files:
  - `README.md`
  - `adr/`
  - `contracts/invariants.txt`
  - `contracts/phase-ownership.md`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `docs/25-machine-input-pdf-improvements-todo.md`
  - `docs/27-vmb-precomposed-math-vector.md`
  - `docs/27-vmb-precomposed-math-vector-todo.md`
  - `schemas/README.md`
- Deliverables:
  - ADR-0033〜0036のclosed判断を変更せず、producer-composed vectorをversioned別経路として拡張する新しいAccepted ADR。
  - `MI4-V03`〜`MI4-V19`と`MI4-13`のdependencyを反映したmaster plan。
  - contract/profile/resource/accessibility/capabilityのpublication sequence。
- Tasks:
  1. `/usr/bin/git`でrepository状態と`MI4-13` statusを確認し、1.3/1.4の公開状態を証拠として記録する。
  2. `adr/README.md`で次に空いているADR番号を直列に予約する。番号をこのtask文書から推測しない。
  3. 1.4未公開なら設計§3の全identity、4 kind、`svg-safe-2`、metric/style/flow、resource-set `/2`、language/navigation `/2`、tagged-PDF `/2`を採択する。1.4公開済みなら§1.3の分岐に従い新contract/profileへ本書を改訂する。
  4. `svg-safe-1`、native math、book-navigation `/1`、tagged-PDF `/1`、public seven-profile bytesのimmutabilityをcompatibility tableへ記録する。
  5. full closed accepted/rejected SVG syntax、media-by-kind、metric relation、spacing、block property、equation-number、overflow、limit charge、diagnostic、dedupe order、alternative/language/structure mappingをADRへ固定する。
  6. `docs/25-machine-input-pdf-improvements-todo.md`へ`MI4-V01`〜`MI4-V19`のID/status/dependencyと本task planへのlinkだけを持つ短いstubを登録し、`MI4-13`のDepends onへ`MI4-V19`を加える。詳細task/acceptanceをmasterへ複製せず、`MI4-11` / `MI4-12`のscopeも変更しない。採用後はmasterがrelease status/dependency、本書が詳細task/acceptanceのownerになる。
  7. Accepted ADRに伴うREADME support matrixと`contracts/invariants.txt`の次の連番invariantを追加し、capability/resource/profileの非公開stagingとatomic publication条件を同じ文言へ揃える。
  8. roadmap、contract matrix、checklist、phase ownership、capability contract、Schema registry docsを同じtarget状態へ更新する。
  9. ADR結論が本書のID、primary files、dependency、公開単位を変えた場合、`MI4-V03`開始前に本書を更新してdocument reviewを再実行する。
- Acceptance criteria:
  - 実装者が新旧contract/profile分岐、全identity、accepted subset、error、limit、fallback禁止を追加判断せず実装できる。
  - master dependency graphが`MI4-V19 -> MI4-13`を持ち、循環せず、JPEG/CFF milestoneを横取りしない。
  - masterと本書のowner分担が明示され、同じ詳細task/acceptanceの複製がない。
  - README support matrix、normative invariant、capability contractがAccepted ADRと一致する。
  - current 1.3/public capabilities/default profileは変更されていない。
  - ADRがAcceptedになるまで`MI4-V03`以降を開始しない。
- Verification:
  - `rg -n 'svg-safe-2|precomposed-vector|math-vector-flow|safe-vector/2|book-navigation.*\/2|tagged-pdf.*\/2|MI4-V19' README.md adr contracts docs/21-roadmap.md docs/22-contract-matrix.md docs/23-implementation-checklist.md docs/25-machine-input-pdf-improvements-todo.md schemas/README.md`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V02 change set containing this completion record.
  - `/usr/bin/git status --short --branch`、current contract constant、top-level Schema alias、canonical capability fixture、public CLI isolation test、master milestoneを照合し、current/publicは1.3・exact seven profiles・default `paragraph-1`、1.4/`production-book-1`はprivate、`MI4-13`はPendingと確認した。`adr/README.md`の連番から`ADR-0037`を採番した。
  - ADR-0037は4 kind、`svg-safe-2`、fixed-point metric/baseline、inline/block layout、Safe-SVG 2、Form/ExtGState/content-key dedupe、alternative/language/structure、limit/diagnostic、capability projectionと全versioned identityをAcceptedにした。`svg-safe-1`、native math、SafeVector/navigation/tagged-PDF `/1`、ADR-0036 resource-set `/1`、public seven-profile bytesはcompatibility tableで凍結した。
  - masterへMI4-V01〜V19のstatus/dependency/linkだけを登録し、`MI4-13`を`MI4-V19`依存にした。詳細task/acceptanceは本書、release status/dependencyはmasterのownerとし、MI4-11 JPEG / MI4-12 CFF scopeは変更していない。
  - 指定`rg`、`python3 schemas/validate.py`、document dependency/identity checks、`/usr/bin/git diff --check`はいずれもexit 0。evidenceはADR-0037、更新したcontract/docs/Schema registry documentation、および本completion recordにある。
  - Rust、private Schema shape、current/public contract/profile/capability bytesは変更していない。scope deviationはなく、新規採択判断はADR-0037だけである。
- Non-goals:
  - Rust実装、private Schema shape、public alias切替

### MI4-V03 Strict Wire、Schema、domain modelを実装する

- Status: Completed
- Depends on: MI4-V02
- Design inputs: docs/27 §4、§13、§14
- Primary files:
  - `workspace/crates/typaxis-document-package/src/semantic_container.rs`
  - `workspace/crates/typaxis-document-package/src/jcs.rs`
  - `workspace/crates/typaxis-document-package/src/lib.rs`
  - `workspace/crates/typaxis-document/src/semantic_container.rs`
  - `workspace/crates/typaxis-document/src/book_navigation.rs`
  - `workspace/crates/typaxis-document/src/lib.rs`
  - `workspace/crates/typaxis-syntax/src/book_navigation.rs`
  - `workspace/crates/typaxis-syntax/src/semantic_container.rs`
  - `workspace/crates/typaxis-syntax/src/tagged_structure.rs`
  - `workspace/crates/typaxis-layout/src/semantic_container.rs`
  - `workspace/crates/typaxis-layout/src/safe_vector.rs`
  - `workspace/crates/typaxis-machine-profile/src/semantic_container.rs`
  - `workspace/crates/typaxis-machine-profile/src/safe_vector.rs`
  - `workspace/crates/typaxis-manifest/src/semantic_container.rs`
  - `workspace/crates/typaxis-manifest/src/safe_vector.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `schemas/1.4/common.schema.json`
  - `schemas/1.4/document-package.schema.json`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - `svg-safe-2` declaration/provenance DTOとtyped domain。
  - 4 kind、metrics、spacing、equation numberのstrict Wire/domain model。
  - 1.4-only canonical decode/encode/JCS round trip。
- Tasks:
  1. `WireImageMediaType` / `ImageMediaType`へ`SvgSafe2`を追加し、`WireVectorProvenance` / `VectorProvenance`を`engine_id`、`engine_version`、`rules_version`のclosed recordとして追加する。
  2. `media_type = svg-safe-2`では`expected_sha256`をrequired nonnull lowercase 64-hex、provenanceをrequiredとし、他mediaではprovenanceをforbidするSchema conditionalを追加する。wire rootの既存required nullable member shapeは保つ。
  3. `WirePrecomposedVectorMetrics` / domain counterpart、viewport、spacingを追加し、JSON integer/tag/unknown-member closureをSchemaとSerdeの両方で一致させる。
  4. `WireStagingM4Inline`へ`InlineVector` / `MathVector`、`WireStagingM4Block`へ`VectorFigure` / `MathVectorBlock`を追加し、domain enumへlossless lowerできる全fieldを持たせる。
  5. `math_vector` / `math_vector_block`は`source_tex` required、`inline_vector` / `vector_figure`は`source_tex` forbiddenとする。`inline_vector`、`math_vector`、`math_vector_block`の`actual_text`はrequired nullable、`vector_figure`ではforbidden、math blockの`equation_number`はrequired nullableとする。`caption`、`metrics` / `viewport`を含むrequired/null/forbidden matrixを§2.3どおりSchema conditionalで閉じる。
  6. equation numberを独立NodeId、SourceSpan、TextSpan、positive `minimum_gap`を持つrequired-nullable recordとしてmodel化する。nonnullだけがowner直後のsource-owned leaf childを一つ持ち、nullはNodeIdを消費しないshapeをdomain traversalへ公開する。
  7. new language-capable kindをdocument-level exhaustive enumへ追加するが、computed-language `/1` encoderへ混ぜない。4 kindをauthored-content判定でnonemptyとし、vector-only paragraphやcaption/番号なしblockをdrop/rejectしない。
  8. Wire/domain/media enumをmatchする全workspace consumerを同じchange setで列挙・更新する。V04以降のreceiptをまだ発行できないconsumerにはowner-specific typed terminal staging errorのexplicit armを置き、silent ignore、既存Figure/native mathへのprojection、dummy receiptを禁止する。
  9. exact field orderingをJCS testで固定し、escaped duplicate key、unknown kind/member、wrong tag/type、missing/null/extra conditional fieldをinvalid fixture化する。
  10. current/frozen 1.0〜1.3 Schema、public decoder、既存1.4 semantic/math/vector fixtureのgolden bytesが変わらないregressionを追加する。
- Acceptance criteria:
  - 全4 kindがcanonical round tripで全fieldをlosslessに保持する。
  - invalid conditional shapeはexact JSON Pointerの`P1102`となり、resource openを開始しない。
  - same image IDのduplicate declarationはresource open前に拒否される。
  - new kind/mediaはprivate 1.4 decoderだけが受理し、public current inputは引き続き拒否する。
  - `source_tex`やprovenanceの禁止branchでunknown fieldとして見逃さずterminal errorになる。
  - 全exhaustive consumerがexplicit new-kind/media armを持ち、workspace全target/featureがこのincremental milestone単独でcompileする。
  - 4 kindだけをauthored contentに持つcontainerが保持され、後続未実装phaseへ到達した場合はtyped staging errorになる。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package precomposed_vector_wire --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document precomposed_vector_domain --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax precomposed_vector_staging_dispatch --locked`
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V03 change set containing this completion record.
  - private contract 1.4へ`svg-safe-2`とclosed provenance、4 vector kind、fixed-point viewport/metrics/spacing、TeX TextSpan、nullable equation-number childを追加した。strict decoder/encoderは条件付きrequired/forbidden field、nonnull lowercase hash、duplicate image ID、unknown/escaped memberをresource open前のexact `P1102` JSON Pointerで拒否し、canonical JCS round tripで全fieldを保持する。
  - domainは4 kind、inline owner、language override、caption、equation-number NodeId/SourceSpan/TextSpanをlosslessに保持する。nonnull numberだけをglobal dense preorderのleaf childとして数え、nullはNodeIdを消費しない。vector-only semantic contentとvector figure caption内の既存semantic subtreeも保持する。
  - computed-language、tagged structure、legacy semantic/SafeVector profile、layout、resource admission、manifest、artifact exporterの全既存`/1` consumerにnew kind/mediaの明示的typed staging errorを追加した。`svg-safe-1`へprojectionせず、Safe-SVG 2 parse、resource open、metric relation/aspect、layout/PDFを先取りしていない。
  - canonical private fixtureは4 kindと`svg-safe-2` provenanceを一つのdense treeで覆い、Schema validatorはprivate 1.4 acceptance、current 1.3 rejection、source/resource hash closure、missing/null/extra/wrong-type負例を検証する。current/frozen 1.0〜1.3 Schema、public seven-profile capability、既存private fixture bytesは変更していない。
  - milestone指定の3 targeted test、workspace all-target/all-feature check、`python3 schemas/validate.py`、workspace全test、clippy `-D warnings`、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。レビューではmanifest/profileの`svg-safe-1` new-kind拒否、vector caption semantic subtree、nested recordのexact JSON Pointer、provenance lexical shape、`/1` language precheck、JSON Pointer escape、clippy guardを修正し、再検証後のfindingは0件である。
  - listed primary file外ではexhaustive error mappingのため`typaxis-syntax/src/lib.rs`と`typaxis-cli/src/pipeline.rs`、Schema corpus gateのため`schemas/validate.py`、status/registry説明のため本書・master・`schemas/README.md`・corpus READMEだけを変更した。責務拡張、新規ADR、public alias/capability switchはない。
- Non-goals:
  - resource bytesのopen、SVG parse、metric relation/aspect validation
  - public contract/current Schema alias切替

### MI4-V04 Metric、source、alternativeのsealed syntax validationを実装する

- Status: Completed
- Depends on: MI4-V03
- Design inputs: docs/27 §4.2〜4.4、§5、§8.4、§10、§13
- Primary files:
  - `workspace/crates/typaxis-syntax/src/semantic_container.rs`
  - `workspace/crates/typaxis-syntax/src/book_navigation.rs`
  - `workspace/crates/typaxis-syntax/src/tagged_structure.rs`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-diagnostics/src/`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
- Deliverables:
  - sealed `ValidatedPrecomposedVectorMetrics` receipt。
  - exact source TeX / Alt / ActualText / language / equation-number validation。
  - resource-independent errorとone-time text/AST charge。
- Tasks:
  1. `typaxis.precomposed-vector-metrics/1`へbindするnon-cloneable receiptを追加し、contract/package/session/limits、NodeId、kind、image ID、全raw metric、canonical JCS、fingerprintをowner-controlled constructorだけで発行する。profile authorizationは`MI4-V08`のjoinまで先取りしない。
  2. §2.4のscalar range/relation、checked subtraction/additionを検査する。intrinsic ratio/scaleはadmitted IRを必要とするため`MI4-V08`で再照合できるようunresolved resource bindingをtypedに保持する。
  3. source_tex TextSpanのnonempty UTF-8、BOM/NUL、identity TextMap、owner SourceSpan containment、exact slice hashを検証し、parse/trim/normalizeしない。
  4. Altとnonnull ActualTextのmeaningful scalar/control規則、math null fallback、inline Figure null absenceをtyped resolved alternativeとして分離する。
  5. provenance各stringのnonempty printable ASCII / 128-byte上限を検査し、parser選択に使わずbinding inputへ保持する。
  6. equation-numberのglobal dense typed-preorder NodeId（owner直後）、leaf depth、唯一のsource-owned child、identity TextMapが対応付けるformula/number SourceSpanのsame-source owner containment、`formula_source_span.end_byte <= equation_number.span.start_byte`、meaningful text、positive gapをlayout前に検査する。nullではchild/NodeId/AST chargeがないことも検査する。
  7. 4 kindのoptional language overrideを既存BCP 47 parserでcanonicalizeし、`/2` registryへ渡すsource-owned recordを作る。equation numberを別language ownerにしない。
  8. TeX、Alt、nonnull ActualText、language、number text、semantic/equation nodeを§2.8のownerへmax+1前にexactly once課金する。
  9. field別`P1102` location、existing text-map error、`P1120/P1121/T2100/T2101`をpositive/exact/max/max+1/tamper testで固定する。
- Acceptance criteria:
  - missing、zero、negative、overflow、baseline外、ascent/descent不足が該当member locationで拒否される。
  - authored bytesとresolved ActualTextを区別し、null fallbackでAlt bytesを再課金しない。
  - sealed receiptをraw field/fingerprintからcallerが構築・swapできない。
  - source validation失敗時にresource open、layout、PDFを開始しない。
  - 番号あり/なしを混在させてもsource/generated NodeId全体がdense preorderとなり、formula-first source/structure順と一致する。
  - current computed-language `/1` recordとgolden bytesは変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax precomposed_vector_metrics --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax precomposed_vector_alternative --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax precomposed_vector_limits --locked`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V04 change set containing this completion record.
  - `typaxis.precomposed-vector-metrics/1`のnon-cloneable receiptをsyntax ownerだけが発行する。receiptはprivate session identity、contract 1.4、canonical package SHA-256、effective limits、owner NodeId/SourceSpan、4 kind、unresolved image binding、全raw metric/spacingまたはviewport、deterministic canonical JCS/fingerprintへbindし、別parseの同一bytes receipt、別package、tamperを`I9190`で拒否する。SVG intrinsic ratio、uniform scale、media/profile authorizationはtyped unresolved bindingのままV08へ残した。
  - metricはpositive/nonnegative/signed safe-integer rangeに加え、baseline、ascent、descent、viewport、`origin_x + viewport.width`をcheckedに検証する。invalid scalar/missing memberはstrict decoderのexact JSON Pointer、関係違反はtyped field location付き`P1102`でresource lowering前に停止する。
  - math source TeXとequation-number textはnonempty UTF-8 exact slice、per-buffer bound、exactly-one identity TextMap、owner containment、TextBuffer/full-slice SHA-256へbindする。TeXはBOM/NULだけを追加拒否してparse/trim/normalizeせず、番号はmeaningful/control、positive gap、owner直後のdense leaf NodeId、formula-first same-source span順を検証する。null番号はNodeId/AST chargeを消費しない。
  - 4 kindのAlt、nonnull ActualText、optional BCP 47 language overrideをexact bytesのまま検証した。math null ActualTextはAlt alias、inline Figure nullはabsenceというtyped resolutionを保持し、fallbackを再課金しない。language source recordはraw/canonical spellingと既課金bytesを保持してcomputed-language `/2`へ渡せるが、既存`/1` registry/bytesは変更していない。`svg-safe-2` provenanceは各値をnonempty printable ASCII / 128 bytesで再検証し、parser selectionには使わない。
  - TextBufferは既存admissionの一回分だけ、Alt/nonnull ActualTextとraw/canonical languageは各authored valueを一回だけaggregateへ課金する。semantic vector/equation nodesは既存AST count/depthを一回だけ使い、`T2100`/`T2101`、`P1120`/`P1121`のexact/max/max+1とreceipt tamperをtargeted mutantで固定した。
  - milestone指定の3 targeted test、`typaxis-syntax`全test/doc-test、workspace all-target/all-feature check/test、workspace clippy `-D warnings`、Schema validator、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。レビューでmalformed TextMapのreversed SourceSpan arithmetic、package全体を先に再検証しないreceipt verification、反復TextSpanがfull limits JCSとslice本文をreceiptごとに複製するmemory amplification、および同じ大きなTextBuffer/sliceのSHA-256を参照回数分再計算するCPU amplificationを検出した。checked subtraction、package closure、一回計算したlimits fingerprint、定数サイズのTextSpan/buffer/slice-hash binding、session-local hash cacheへ修正した後のfindingは0件である。
  - listed primary file外の変更はmaster statusと本completion recordだけである。resource open、Safe-SVG parse、intrinsic ratio/scale、style/profile authorization、layout/PDF、computed-language `/2`発行を先取りしていない。
- Non-goals:
  - SVG intrinsic ratio、Form key、selected placement
  - math semantic equivalenceやreading品質の判定

### MI4-V05 Precomposed vector styleとprivate profile authorizationを実装する

- Status: Completed
- Depends on: MI4-V04
- Design inputs: docs/27 §4.4、§7、§12、§14
- Primary files:
  - `workspace/crates/typaxis-style/src/lib.rs`
  - `workspace/crates/typaxis-syntax/src/semantic_container.rs`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/descriptor.rs`
  - `workspace/crates/typaxis-machine-profile/src/safe_vector.rs`
  - `workspace/crates/typaxis-machine-profile/src/semantic_container.rs`
  - `workspace/crates/typaxis-machine-profile/src/math.rs`
  - `workspace/crates/typaxis-machine-profile/src/tagged_pdf.rs`
  - `workspace/crates/typaxis-machine-profile/src/capabilities.rs`
  - `workspace/crates/typaxis-machine-profile/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/tests.rs`
  - `schemas/1.4/machine-capabilities.schema.json`
  - `docs/25-machine-input-pdf-improvements-todo.md`
- Deliverables:
  - `typaxis.precomposed-vector-style/1`のclosed registry/cascade receipt。
  - private production profileによるkind/media/metric/style authorization。
  - public seven-profile descriptor isolation。
- Tasks:
  1. existing declaration value、specificity、important、source-order、extends engineを再利用しつつ、`math_vector_block` / `vector_figure`だけを受ける別registry identityとcomputed receiptを追加する。
  2. §2.5のproperty applicabilityをexhaustive enum/tableで実装する。property名文字列をlayoutで再判定せず、typed computed fieldsだけを渡す。
  3. equation-number text styleは`math_vector_block`のfont fieldsをconsumeし、formula viewport/metricsには適用しない。`vector_figure` captionは既存caption child styleを使う。
  4. basic registry `/1`へnew selectorを渡すと拒否し、new registry receiptとbasic receiptをswapすると`I9190`となるtestを追加する。
  5. private `production-book-1` preflightへ4 kind、kind別media、required metric names、`svg-safe-2` provenance、SafeVector/resource-set `/2` identityをclosed登録する。
  6. old profileはnew kind/media/styleをresource open前に拒否し、private profileもkind/media mismatch、`math_vector` + `svg-safe-1`、existing `figure` + `svg-safe-2`を拒否する。
  7. capability data modelへ設計§12のvector fieldを表現できるtyped valueを追加する。既存`image_formats`はcoarse `jpeg|png|svg`だけ、Safe-SVG exact profile/mediaはnew vector fieldsだけに出す型境界を固定し、public serializer/profile tupleへはまだ接続しない。private projection testだけを持つ。
  8. property exact/min/max/max+1、inherit、override、inapplicable、wrong registry、profile rejectionをfixture化する。
- Acceptance criteria:
  - new block kindの全propertyがexactly one typed consumerを持つ。
  - `width`およびmathの`keep_caption`は`L5101`、unknown selector/propertyはlayout前に拒否される。
  - private authorizationのaccepted setと§2.3 matrixが双方向に一致する。
  - `typaxis capabilities --format json`、public schema/fixture bytes、default profileは変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-style precomposed_vector_styles --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile precomposed_vector_profile --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile public_capability_isolation --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V05 change set containing this completion record.
  - `typaxis.precomposed-vector-style/1`をbasic `/1`とnominally分離し、`math_vector_block` / `vector_figure`だけを受けるselector、全property applicability/initial/inheritance/typed consumer table、既存specificity・`important`・source-order・`extends` engineを再利用するsealed computed receiptを追加した。receiptはregistry、kind、typed block fields、named page、equation-number text styleまたはcaption keep、canonical JCS/fingerprintをbindし、kind/supplement、fixed `width:auto`、math caption initialも再検証する。
  - syntaxはmixed staging stylesheetを一度strict検証した後、basic/semantic/math registryとvector registryへ閉じて分離し、cross-registry `extends`を拒否する。vector block ownerごとのreceiptをvalidated packageへpointer-boundで保持し、`vector_figure` captionはownerのvector alignment/fontを継承せず既存caption child styleを独立解決する。`width`、math `keep_caption`、vector Figure ownerのfont tripleはlayout前の`L5101`、unknown selector/propertyはterminal style errorとなる。
  - private preflightは4 kindのmetric receipt、kind別`svg-safe-1|svg-safe-2` matrix、block style receipt、Safe-SVG 2 provenance、`typaxis.resource-profile/safe-vector/2`、`typaxis.production-book-resource-set/2`を一つのsession/limits/package-bound canonical receiptへ閉じた。既存FigureはSafe-SVG 1だけ、math vectorはSafe-SVG 2だけを受理し、旧semantic/SafeVector `/1` profileはnew kind/media/styleをresource open前に拒否する。`vector_figure` caption subtreeも既存Figureと同じ`FigureCaption` domainで再帰検証・semantic countする。
  - capability modelへcoarse `jpeg|png|svg`とvector block/inline/kind/profile/media/metric/featureのnominal typeを追加した。test-only private projectionを設計§12 exact JCSおよび実preflight matrixと双方向照合した一方、public serializer、seven-profile tuple、default `paragraph-1`、1.4 capability Schema、public capability fixture bytesには接続していない。
  - milestone指定のtargeted test、changed crate全test/doc-test、workspace all-target/all-feature check/test、workspace clippy `-D warnings`、fmt check、`python3 schemas/validate.py`、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは3869 refsを含む全bundle/fixtureを通過した。
  - レビューでは`vector_figure` captionをprofile domain walkが再帰しない迂回、vector styleにも旧`semantic_container`名を出す診断、receipt kind/supplement不一致をself-consistent fingerprintで閉じない不変条件、capability projectionとpreflight media matrixのdrift余地、platform-dependent fixture testのguard、clippy findingを修正した。再検証後のfindingは0件である。
  - listed primary file外の変更は本completion recordだけである。`math.rs`、`tagged_pdf.rs`は共有する旧profile拒否で閉じるため変更せず、Schema/sample/public capability bytes、新style color、SVG parse/layout/PDF処理を先取りしていない。
- Non-goals:
  - public capability advertisement
  - SVG paint colorをauthorする新style property

### MI4-V06 Safe-SVG 2のbounded admissionとcanonical IRを実装する

- Status: Complete
- Depends on: MI4-V03
- Design inputs: docs/27 §8、§9.1、§13
- Primary files:
  - `workspace/crates/typaxis-resource-admission/src/safe_vector.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/Cargo.toml`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - `typaxis.safe-svg-parser/2`のclosed iterative parser。
  - paint kind/alphaを持つ`typaxis.safe-vector-ir/2`とattestation。
  - Safe-SVG 1 byte/fingerprint isolationとSafe-SVG 2 negative corpus。
- Tasks:
  1. parser profileをnominal enumで分け、`svg-safe-1`は既存`SAFE_SVG_PARSER_ID` / IR `/1`を、`svg-safe-2`だけはnew `/2` identityを選ぶ。boolean feature flagやmedia文字列の下流再判定にしない。
  2. canonical paintを`None | FixedRgb8 | CurrentColor`へし、fill/strokeそれぞれにresolved unsigned 16.16 alphaを保持するversion-2 IR recordを追加する。version-1 IRのfield/JCSを変えない。
  3. exact ASCII `currentColor`だけをfill/stroke valueとして受理し、case alias、surrounding whitespace、`inherit`、`var()`、SVG `color`を拒否する。
  4. `fill-opacity` / `stroke-opacity`を`g`とpaint geometryのpresentation attributeだけで受理する。lexicalはexact `0`、`1`、`0.` + 1〜6 digit、`1.` + 1〜6 zeroに閉じ、round-half-to-evenで16.16へ変換する。
  5. opacityはsource nestingでinheritし、child specified値で置換する。乗算、group/object `opacity`、clip geometry alpha、mask/soft mask/blend/isolationを拒否する。
  6. existing path/transform/viewBox/clip rulesをreuseし、root viewport clipを最外周へ固定する。unknown/unused/cyclic/forward/external clip reference、およびscript/event/animation/foreignObject/image/text/font/CSS/href/use/symbol/gradient/pattern/filter/entity/DOCTYPE/PI/unknown element/attributeをtyped reasonで拒否する。
  7. version-2 allocation chargeをchecked式`64 * nodes + 80 * stored_segments + 48 * paint_or_clip_commands + source_clip_id_bytes`へ変更し、issue/allocation前に既存inclusive limitをconsumeする。version-1式は凍結する。
  8. fill/stroke双方がdisabledまたはenabled paintのalphaが全てzeroとなるresourceを`R7100 forbidden_feature`相当のclosed reasonで拒否する。
  9. same admitted SHA-256でfull stable bytesが異なるresource aliasをledger completion時に`resource_conflict`として拒否する。productionはstable full bytesから実SHA-256を計算してdeclared hashを先に照合し、collision guardだけはowner-private/test-only injected admitted-record/digest seamで同一digest・異なるbytesを与えて検証する。偽の実SHA collision fixtureやpublic digest overrideを作らない。
  10. positive corpus、exact/max/max+1、malformed XML/path、全forbidden feature、alpha lexical/range/inheritance、clip forward/cycle/external、declared hash mismatch、test-only collision guard、old `/1` goldenをtestする。
- Acceptance criteria:
  - Safe-SVG 2のunknown/unsupported syntaxを一件もskipして成功しない。
  - parserはfilesystem、network、font、locale、platform SVG rendererへアクセスしない。
  - admitted recordがstable byte hash、media、parser/IR ID、IR fingerprint、limit/profile fingerprintをbindする。
  - Safe-SVG 1のIR canonical JCS/fingerprint、allocation charge、fixture bytesが変更前と一致する。
  - invalid inputはpanic、recursive stack growth、unbounded allocationを起こさずresource admissionで終端する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission safe_svg_2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission safe_svg_1_frozen --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit vmb_safe_svg_negative_corpus --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V06 change set containing this completion record.
  - declared `ImageMediaType`から選ぶnominal `SafeVectorParserProfile`を追加し、既存Safe-SVG 1 decoder/JCS/fingerprint/allocation式を変更せず、独立したfixed-stack 3-pass Safe-SVG 2 decoderと`typaxis.safe-vector-ir/2`を実装した。`None | FixedRgb8 | CurrentColor`とfill/stroke別unsigned 16.16 alpha、root viewport最外周clip、parser/IR/fingerprint/allocation identityをcanonical IRへ保持する。
  - Safe-SVG 2はexact `currentColor`とclosed alpha lexicalだけを受理し、alphaをround-half-to-evenで変換してsource nestingでは継承、child指定では置換する。既存geometry/path/transform/viewBox/local clip規則を再利用し、CSS、script/event/animation、text/font/image/use、external reference、paint server、group opacity、mask/filter/blend、entity/DOCTYPE/PI、unknown syntaxをtyped `R7100` reasonでfail closedにした。positive paintが全drawに存在しないresourceも`forbidden_feature`で拒否する。
  - Count passでnode/path/depthのinclusive limitとchecked `64 * nodes + 80 * stored_segments + 48 * paint_or_clip_commands + source_clip_id_bytes`を検証してからAnalyze/Build allocationへ進む。stable full bytesから実SHA-256を計算してdeclared hashとparser work前に照合し、ledger completionではdigestごとの先頭bytesとのlinear full-byte比較でcollision aliasを`resource_conflict`へ閉じた。任意digestを注入できる面はcrate-private test seamだけである。
  - admitted image/declared-media attestationは`svg-safe-2`、parser/IR ID、IR fingerprint、allocation charge、stable hash、M4 limits/profile fingerprintをbindする。legacy `safe_vector()` APIはSafe-SVG 1だけを返し、new V2 accessorsをnominally分離した。Safe-SVG 1はcanonical IR JCS hash、IR fingerprint、allocation charge、checked-in fixture SHAをgoldenで固定した。
  - checked-in positive VMB corpus全件と、4 typed reasonを覆うsorted `negative.tsv` / `negative-svg/` corpusを検査する。negative ledgerのrow/path/reasonをparserのexact resultへ直接結び、alpha lexical・inherit/replace、clip closure、limit exact/max+1、hash mismatch、private collision、truncated/non-UTF-8/deep nesting no-panicを追加した。
  - milestone指定のtargeted test、changed crate test、workspace all-target/all-feature test、workspace clippy `-D warnings`、fmt check、`python3 schemas/validate.py`、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは3869 refsを含む全bundle/fixtureを通過した。
  - レビューではnesting limit/no-panicの明示的境界不足、negative ledgerとtyped reason期待値が独立してdriftできる点、同一digest aliasのcollision guardが全ペアfull-byte比較となる二乗時間経路を修正した。再検証後のfindingは0件である。
  - listed primary file外ではexhaustive enum compatibilityのため`typaxis-cli` 2ファイルと本completion recordを変更した。`typaxis-resource-admission/Cargo.toml`は変更せず、新規SVG/XML/renderer dependency、Display/layout/PDF/Form処理を追加していない。
- Non-goals:
  - SVG 2/CSSの一般実装
  - Form/PDF object生成

### MI4-V07 VectorContentKeyとdedupe planning primitiveを実装する

- Status: Completed
- Depends on: MI4-V06
- Design inputs: docs/27 §8.3、§9、§11
- Primary files:
  - `workspace/crates/typaxis-resources/src/safe_vector.rs`
  - `workspace/crates/typaxis-resources/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
- Deliverables:
  - typed `VectorContentKey`と`typaxis.vector-form-dedupe/1` receipt。
  - admitted aliasをcontent単位へcanonicalizeするcandidate registry。
  - deterministic ExtGState planning primitive。
- Tasks:
  1. `VectorContentKey`を32-byte source hash、typed media、parser ID、IR ID、32-byte IR fingerprintのnominal tupleとして実装する。public raw tuple constructorを持たせずadmitted recordからだけ作る。
  2. admitted SafeVector aliasをcontent keyでgroup化する`VectorContentCandidateRegistry`相当のsealed registryを作る。source hashだけ、IR fingerprintだけ、文字列連結だけでdedupeしない。
  3. key comparisonをtuple component順へ固定し、candidate orderをowner-private admitted-record列挙順、hash-map insertion、worker completionから独立させる。wire declaration順/dense image ID自体は書き換えない。
  4. content candidateにcanonical IR、intrinsic size/viewBox、source/media/parser/IR identity、sorted ExtGState alpha pairs、all alias IDsをbindする。selected usage、Form object/name、PDF hashはまだ持たせない。
  5. alias recordにimage ID、declared URI/expected hash、conditional provenance、admitted hash/profile/limitsを保持する。provenanceをForm keyへ含めず、alias別usage countはDisplay join後の`MI4-V13`だけが確定する。
  6. ExtGState pairはfill/stroke unsigned 16.16のnumeric昇順で一意化し、alpha 1/1も含める。Form-local nameとrelative object-role orderは後続PDF ownerがこの順だけから決められるようにし、absolute object numberはまだ持たせない。
  7. selected usageとcandidateをjoinするprivate input typeを定義するが、Display `/2`を先取りしてconstructせず、final `safe-vector-form-plan(s)/2`は`MI4-V13`だけが発行する。
  8. dedupe後のForm/ExtGState relative object-role count deltaをchecked計算できるAPIを用意するが、global `max_pdf_objects`はconsumeしない。resource byte/IR admission chargeはaliasごとに免除しない。
  9. existing `finalize_staging_safe_vector_forms`と`typaxis.safe-vector-form-plan(s)/1`の結果を変えず、new candidate/planning typeをnominally分離する。
  10. same ID/different content、same key/two IDs/different provenance、same hash/different media、same IR/different source hash、unused alias、同一alias recordsのowner-private candidate/worker order permutationをtestする。
- Acceptance criteria:
  - same content keyのN aliasはexactly one content candidateになる。
  - same SVG hashでもmedia/parser/IR identityが異なれば別candidateになる。
  - alias provenanceがdedupe候補化後も失われない。
  - wire input/alias recordsを固定したinternal candidate/worker order permutationでもcandidate JCS/fingerprint/orderが一致する。
  - candidate receiptはrelative object-role/countだけを持ち、absolute object numberまたはglobal PDF-object chargeを先取りしない。
  - `/1` finalizer、Figure fixture、canonical bytesは変更前と一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resources vector_content_key --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resources vector_content_candidates --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resources vector_ext_gstate_plan --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resources safe_vector_form_plans_v1_frozen --locked`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V07 change set containing this completion record.
  - admitted stable-byte SHA-256、vector-only typed media、parser ID、IR ID、IR fingerprintのexact tupleをprivate fieldへ持ち、`AdmittedImage`以外から作れない`VectorContentKey`を追加した。比較はsource hash、media UTF-8、parser ID UTF-8、IR ID UTF-8、IR fingerprintのcomponent順で明示実装し、文字列連結、resource ID、provenance、first-use順をkeyへ混入させていない。
  - `VectorContentCandidateRegistry`はcomplete declared-media closureを再検証してから全SafeVector aliasをkey・numeric image ID順へcanonicalizeする。同一keyはcanonical admitted IR、intrinsic size/viewBox、numeric unique alpha pair planを共有する一候補となり、異なるsource hashまたはmedia/parser/IR identityは共有しない。`typaxis.vector-form-dedupe/1` receiptはcandidate/alias count、candidate facts、conditional relative object-role delta、canonical JCS/fingerprintをbindする。
  - alias recordはimage ID、declared URI/expected hash、admitted hash、per-alias IR allocation charge、profile/limits fingerprintを保持する。Safe-SVG 2だけproducer engine/version/rules provenance memberをrequired objectとして保持し、Safe-SVG 1ではmember自体を出さない。同一content aliasもadmission chargeを各recordへ残し、unused aliasもfactから落とさない。
  - ExtGState planは各admitted drawのresolved fill/stroke unsigned 16.16 pairだけをnumeric昇順で一意化し、実在するopaque `(65536,65536)`も省略しない。Form roleをrelative zero、ExtGState roleをpair順のone-based値とし、checked Form + ExtGState countだけを発行する。absolute object number、resource name、global `max_pdf_objects` chargeは発行していない。
  - selected candidateとalias別nonzero usage countのprivate join shapeだけを予約し、public constructor、selected usage、Form plan/name、PDF hashを追加していない。既存`finalize_staging_safe_vector_forms`と`typaxis.safe-vector-form-plan(s)/1`は変更せず、既存Figure fixtureのexact canonical JCS/fingerprintを`safe_vector_form_plans_v1_frozen`で固定した。
  - same ID/different content、same key/two IDs/different provenance、same admitted hash/different Safe-SVG media/parser/IR、same canonical IR/different source hash、unused alias、conditional provenance/hash failure、alias/candidate worker order permutation、alpha pair unique/order/opaqueを実admission corpusとowner-private permutation seamで検証した。
  - milestone指定の4 targeted test、changed crate test/doc-test、workspace all-target/all-feature test、workspace clippy `-D warnings`、fmt check、`python3 schemas/validate.py`、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは3869 refsを含む全bundle/fixtureを通過した。
  - レビューでは非opaque drawだけの候補へ未使用opaque ExtGStateを無条件追加する過剰計上、Safe-SVG 1 provenanceをabsentでなくnull memberとしてencodeするconditional違反、およびclippy findingを修正した。再検証後のfindingは0件である。
  - listed primary file外ではnew nominal implementation module `workspace/crates/typaxis-resources/src/vector_content.rs`と本completion recordを追加した。`typaxis-testkit`は既存VMB positive corpusを`typaxis-resources`の実admission testから再利用できたため変更せず、Display `/2`、selected usage finalization、Form/PDF serialization、Schema/public capabilityを先取りしていない。
- Non-goals:
  - selected usage finalization、PDF object numberの発行、Form stream serialization
  - source hashが異なる同一IRのdedupe

### MI4-V08 Resource-aware metricとsource/vector bindingを閉じる

- Status: Completed
- Depends on: MI4-V04, MI4-V05, MI4-V06, MI4-V07
- Design inputs: docs/27 §4.2、§5、§9.1、§10
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/lib.rs`
  - `workspace/crates/typaxis-layout/src/math.rs`
  - `workspace/crates/typaxis-layout/src/safe_vector.rs`
  - `workspace/crates/typaxis-layout/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/safe_vector.rs`
  - `workspace/crates/typaxis-machine-profile/src/math.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
- Deliverables:
  - common `ValidatedPrecomposedVectorReceipt`とmath専用`ValidatedMathVectorReceipt`。
  - intrinsic ratio/uniform-scale proof、resolved paint、profile/media closure。
  - backend-independent selected-placement input contract。
- Tasks:
  1. syntax metrics receipt、admitted SafeVector attestation、profile/style authorization、package/session/effective limits/LayoutEpochを一つのowner-controlled bindingへjoinする。
  2. checked `i128`とround-half-to-evenで§2.4のsingle scaleを導出し、both-axis exact rounded result、16.16 range、`origin_x + viewport.width`を検証する。nonuniform/ratio mismatchは`P1102`またはprofile preflightの定義済みlocationで終端する。
  3. kind/media matrix、image ID、declared/admitted media、stable SHA、parser/IR ID/fingerprint、provenance、metric receiptをexact照合する。
  4. placement ownerのresolved text paintをtyped RGB8としてbindする。現行authored colorがない場合はexact blackとし、resource IRのCurrentColorを解決済みfixed paintへ書き換えない。
  5. common receiptは4 kindのresource/geometry/paint/alternative/language inputを持ち、math receiptだけがexact TeX span/buffer/slice hash、resolved ActualText、provenance、math kindを`typaxis.precomposed-math-binding/1`へbindする。
  6. inline receiptへspacing、block receiptへcomputed precomposed-vector styleをbindし、inapplicable memberを型として持たせない。
  7. downstream object number、Form resource name、MCID、StructureNodeId、page/line/paint ordinalをbase receiptへ入れない。
  8. `MathComputationReceipt`へexternal metrics用public constructorを追加せず、native mathとnominal type/fingerprintを共有しない。
  9. wrong image/media/parser/IR/profile/limits/epoch/style/source/alternative/paint swapを`I9190`、aspect/scale failureをtyped input errorとしてtestする。
- Acceptance criteria:
  - layoutへraw URI/SVG/TeX、unverified expected hash、float scaleが渡らない。
  - `viewport_top + baseline = line_baseline`を後続layoutが整数演算だけで再構成できる。
  - math bindingからsource/alt/actual/metrics/vector/provenanceのどれかをswapすると検証に失敗する。
  - generic vector receiptにmath-only field、inline receiptにblock-only fieldが存在しない。
  - native math receiptとproducer-composed receiptをAPIまたはserialized identityで取り違えられない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout precomposed_vector_binding --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout precomposed_vector_scale --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout precomposed_math_binding_tamper --locked`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V08 change set containing this completion record.
  - admitted intrinsic sizeからproducer viewportへのscaleを、checked `i128`による一度だけのround-half-to-evenでpositive 16.16へ導出し、同じscaleによる両軸の個別丸め結果をexact照合するbackend-independent placement inputを追加した。inlineはmetrics/spacing、vector Figureとmath blockはkind別computed styleだけを型として保持し、baselineの整数往復、`origin_x + viewport.width`、resolved black RGB8を閉じた。
  - resource admission ownerが発行する`SafeVectorAdmissionAttestation`を追加し、common `ValidatedPrecomposedVectorReceipt`でimage ID、declared/admitted Safe-SVG media、stable source SHA-256、nominal parser profile、parser/IR/fingerprint identity、intrinsic geometry/viewBox、profile/limits、syntax metric/style receipt、alternative/languageを一つのlayout epochへjoinした。raw URI/SVG、caller expected hash、float scale、page/Form/object/MCID stateはreceiptへ渡していない。
  - private production profile receiptからsession-bound `StagingPrecomposedVectorProfileAuthorization`を発行し、package/semantic/limits、全vector owner/metric fingerprint、full profile receipt fingerprintをlayoutとresource admissionへ引き渡すdependency-inversion境界を追加した。process-local profile/admission progress tokenはcanonical bytesへ含めず、同一inputのbinding JCS/fingerprintが別sessionでも一致することを検証した。
  - math kindだけにnominal `ValidatedMathVectorReceipt`とexact `typaxis.precomposed-math-binding/1`を追加し、common vector fingerprint、inline/block kind、TeX TextSpan/mapped SourceSpan、TextBuffer/exact slice SHA-256、resolved ActualText、producer engine/version/rulesをbindした。native `MathComputationReceipt` / `typaxis.math-binding/1`は変更せず、producer-composed mathへのconstructorや変換を追加していない。
  - 4 kind、Safe-SVG 1/2混在、uniform-scale丸め、baseline往復、wrong image/declared・admitted media/stable hash/parser profile・ID/IR/profile/limits/epoch/metrics/style/alternative/language/paint、math source span/TextSpan/buffer/slice/vector/ActualText/provenance/kind、foreign profile sessionをpositive/tamper testで固定した。
  - milestone指定の3 targeted test、変更5クレートの全test/doc-test、forbidden dependency guard、workspace all-target/all-feature test、workspace clippy `-D warnings`、fmt check、`python3 schemas/validate.py`、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは3869 refsを含む全bundle/fixtureを通過した。
  - レビューでは`typaxis-layout-contract -> typaxis-document`と`typaxis-layout -> typaxis-machine-profile`の禁止依存をsyntax DTO facade/session-bound authorizationへ反転し、Safe-SVG 1正常系、決定性、component別tamper coverage、MSRV非対応API、clippy findingsを修正した。再検証後のfindingは0件である。
  - listed primary file外ではdependency inversionのため`workspace/crates/typaxis-syntax/src/lib.rs`、`workspace/crates/typaxis-syntax/src/semantic_container.rs`、`workspace/crates/typaxis-machine-profile/src/semantic_container.rs`と本completion recordを変更した。`typaxis-machine-profile/src/math.rs`はnative math isolationにより変更せず、line break、physical block/page placement、Display/PDF/Form、manifest、Schema/public capabilityを先取りしていない。
- Non-goals:
  - line break、page placement、Display/PDF serialization

### MI4-V09 Inline vector itemization、改行、line metricsを実装する

- Status: Completed
- Depends on: MI4-V08
- Design inputs: docs/27 §5、§6、§15.2
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/lib.rs`
  - `workspace/crates/typaxis-linebreak/src/lib.rs`
  - `workspace/crates/typaxis-linebreak/src/math.rs`
  - `workspace/crates/typaxis-linebreak/src/unicode_linebreak.rs`
  - `workspace/crates/typaxis-layout/src/lib.rs`
  - `workspace/crates/typaxis-layout/src/safe_vector.rs`
  - `workspace/crates/typaxis-layout/src/math.rs`
  - `schemas/1.4/layout-trace.schema.json`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - `AtomicVectorInlineItem`と`VectorBoundaryItem`。
  - `advance`、conditional spacing、dynamic ascent/descentを使うline selection。
  - inline selected placement/trace under `typaxis.atomic-vector-inline/1` / layout `/1`。
- Tasks:
  1. each `inline_vector` / `math_vector`を内部break候補を持たないexactly one atomic itemへlowerし、source provenance付きsynthetic AL unit、atomic LTR isolateとしてUnicode item列へ参加させる。source textへU+FFFCを挿入しない。
  2. vector前後boundaryを既存Unicode rule + Japanese pair tableが決めたBreakKind/penaltyと、no-break branchだけが持つfixed `same_line_width`へlowerする。spacingを裸Glueや独立break candidateにしない。
  3. vector boundaryではJapanese natural gap/stretch/shrinkを加算せず、specified spacingをexact total zero-stretch/zero-shrink gapとして使う。
  4. line頭before、line末after、break selected boundaryの両側gapをzeroにする。adjacent vectorではleft.after + right.beforeを一つのlogical boundaryへexactly once加算する。
  5. break width/costはadvanceを使い、final feasibilityはlogical advanceと`origin_x .. origin_x + viewport.width`の両方をframe boundsへ照合する。
  6. candidate lineのascent/descentをtextと全atomic vectorのmaxで求め、computed line-heightとの差をround-half-to-evenでbefore/after leadingへ配る。pagination line advanceへ同じheightを渡す。
  7. current lineにfitせずempty next lineにfitするvectorはwhole itemを移す。empty lineでもlogical/visual widthがfitしない、またはempty full frameにもdynamic line heightがfitしない場合は`L5100`にする。
  8. selected occurrenceをcontaining fragmentとは別に`max_fragments`へ一回だけ課金し、NodeId、line/page/frame、pen origin、baseline、viewport、scale、spacing decision、paint ordinalをlayout `/1` receiptへbindする。
  9. existing native `AtomicMathInlineItem`とmath line breakingのgoldenを変えず、vector-specific receiptとのnamespace swapを拒否する。
  10. fraction/sum/integral/subscript/matrix same-line、日本語/句読点/bracket隣接、line-end、adjacent vector、overhang、height page move、exact/max+1をfixture化する。
- Acceptance criteria:
  - vector内部にfragment/break recordがなく、one node = one atomic item = one selected occurrenceである。
  - 全inline occurrenceで`viewport_top + baseline == line_baseline_y`が成立する。
  - line width factはviewport bboxでなくadvanceを含み、visual overhangも別にframe fit検査される。
  - prohibited Japanese boundaryをpositive spacingがbreakableに変えず、natural gapを二重加算しない。
  - 高いvectorを含むlineと次line/pageのpaint boundsが重ならない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-linebreak atomic_vector_inline --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-linebreak vector_japanese_boundaries --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout inline_vector_layout --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V09 change set containing this completion record.
  - producer-composed vector binding fingerprintをnative math fingerprintとは異なるnominal typeにし、V08で検証済みのinline placementだけから`AtomicVectorInlineItem`を発行する境界を追加した。atomic itemはowner NodeId、paragraph、ゼロ長を含めたSourceSpan、kind、全metric/spacing、uniform scale、resolved paintを`typaxis.atomic-vector-inline/1` fingerprintへbindし、source textへU+FFFCや代替scalarを挿入しないsource-owned synthetic `AL` / atomic LTR isolateとして扱う。
  - typed logical unit列全体へUnicode 16 line-break ruleを適用し、vector隣接境界だけをexactly one `VectorBoundaryItem`へlowerした。Japanese pair tableはpermission/penaltyだけを使い、specified before/afterをno-break branchのzero-stretch/zero-shrink exact total gapとした。line頭/末とselected breakのgapはzero、adjacent vectorはleft.after + right.beforeを一境界で一回だけ加算する。
  - line selectionはlegal candidateのcumulative demeritを最小化し、logical fit/cost/pen advanceには`advance`、最終visual fitには独立した`origin_x .. origin_x + viewport.width`を使う。vector内部のbreak/fragmentを作らず、empty lineにも収まらないlogical/visual extentはNodeId付き`L5100`で終端する。
  - 各selected lineはtext/vectorのmaximum ascent/descent、computed line-heightとの差のround-half-to-even leading、同じpagination advanceを保持する。shared layout-contract geometry ownerが`viewport_top = line_baseline_y - baseline`と`viewport_left = pen_origin_x + origin_x`をchecked整数演算で導出し、全placementで`viewport_top + baseline == line_baseline_y`を再検証する。dynamic lineが残余frameへ入らなければline全体を次pageへ送り、empty full frameより高ければ`L5100`にする。
  - private `typaxis.precomposed-vector-layout/1` selected receipt/traceはpackage、profile、limits、admission、binding set、LayoutEpoch、page geometry、itemization/line-selection fingerprint、line/page/frame/fragment、pen/baseline/viewport/scale、spacing decision、paint ordinalをcanonical JCSへbindする。vector occurrenceを既存line fragmentとは別に`max_fragments`へ一回ずつ予約し、残余line budgetをselectionへ渡して上限超過をline/occurrence record allocation前の`L5110`にした。
  - VMB corpusのfraction-equality、sum、integral、scripts、large-brackets、matrix metricを同一lineで検査し、日本語/句読点/bracket、line-end whole move、adjacent spacing、logical/visual overhang、dynamic page move/full-frame oversize、minimum-demerit choice、empty SourceSpan、exact/max+1 fragment limitをunit/fixture testで固定した。実selected bytesを`inline-layout-trace.json` goldenとprivate 1.4 Schema/validatorへ追加した。
  - milestone指定の4 targeted command、changed pathを含むworkspace全target/all-feature test、workspace clippy `-D warnings`、doc-test、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは3934 refsを含む全bundle/fixtureを通過した。
  - レビューではbreak costを記録するだけでselectionへ使っていなかった点、契約にないnonempty owner SourceSpan制約、selected fragment limitの事後判定、完全性検証のsaturating加算/unchecked index、MSRV 1.75非対応APIを修正した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外では専用implementation module `workspace/crates/typaxis-linebreak/src/vector.rs`、`workspace/crates/typaxis-layout/src/inline_vector.rs`、page geometryを既存authorizationから安全に渡す`workspace/crates/typaxis-syntax/src/semantic_container.rs`、fixture validator、本completion recordを変更した。既存`typaxis-linebreak/src/math.rs` / `typaxis-layout/src/math.rs`とnative `typaxis.math-flow/1` bytesは変更せず、block vector、Display/PDF/Form、accessibility/manifest、public capability/CLI integrationを先取りしていない。
- Non-goals:
  - vertical writing、bidi reorderを含む新profile
  - SVG内部break、automatic scale

### MI4-V10 MathVectorFlowIdとequation-number shapingを実装する

- Status: Completed
- Depends on: MI4-V08
- Design inputs: docs/27 §4.4、§7、§10、§15.2
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/lib.rs`
  - `workspace/crates/typaxis-layout/src/math.rs`
  - `workspace/crates/typaxis-layout/src/semantic_container.rs`
  - `workspace/crates/typaxis-layout/src/lib.rs`
  - `workspace/crates/typaxis-shaping/src/`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
- Deliverables:
  - nominal `MathVectorFlowId`、registry、terminal receipt。
  - nonwrapping equation-number shape receipt。
  - parent flow projectionとnative math isolation。
- Tasks:
  1. `MathVectorFlowId`をnative `MathFlowId`と別newtypeにし、validated documentの`math_vector_block` NodeId preorderからworker起動前に0始まりdense allocationする。
  2. each flow recordへowner NodeId、parent `FlowId` / position、validated math-vector receipt fingerprint、computed style fingerprint、LayoutEpoch、exact terminal 1をbindする。
  3. parent production flowへtyped atomic display-math categoryとして投影しつつ、exact wire kindとproducer-composed receiptをlayout `/1`に保持する。basic flow registry `/1`のvocabularyを広げない。
  4. missing、duplicate、non-dense、wrong owner/parent/position/epoch/terminal、native MathFlowId swapをlayout開始前またはfinish時に`I9190`で拒否する。
  5. equation-number TextSpanをexisting text shaping pipelineでone nonwrapping lineへshapeし、source text、font selection、glyph receipt、computed number styleへbindする。
  6. shape width/heightがpositiveでない、wrapping/second lineが必要、glyph coverageがない場合はfallbackせず`L5100`またはexisting shaping errorにする。
  7. number childはowner math blockのcomputed language fingerprintを参照し、独立language override owner/countを作らない。
  8. number receiptをformula vector/source/alternative bindingから分離し、number textをSVG SHA-256、VectorContentKey、source-TeX slice hash、formula resolved ActualTextへ含めない。
  9. numberなしではNodeId/shape/rectangle/paint/structure childを作らない。numberありではV04が検証したowner直後のdense NodeIdを再採番せず使い、exactly one leaf shapeとchild ownershipをformula-first順で登録する。
  10. native display mathとmath-vector blockを交互にしたfixtureで両ID空間が独立dense、各terminalが一回だけconsumeされることを検査する。
- Acceptance criteria:
  - registration/worker/page orderを変えてもMathVectorFlowIdとregistry fingerprintが一致する。
  - parent flowはpage move中にterminalを消費せず、selected block成功時だけexactly once消費できる。
  - equation numberはproducer exact textを使い、生成・increment・normalizeしない。
  - number childはowner直後のdense source NodeIdを保持し、null/presentで余分なgenerated NodeIdやgapを作らない。
  - native `typaxis.math-flow/1` recordとgolden bytesが変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout math_vector_flow --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout equation_number_shape --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout native_and_vector_math_flow_isolation --locked`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V10 change set containing this completion record.
  - native `MathFlowId`とnominalに異なる`MathVectorFlowId` / typed terminalをlayout-contractへ追加し、`typaxis.math-vector-flow/1` registryへowner、parent FlowId/position、existing DisplayMath projection、exact `math_vector_block` wire kind、producer math binding、computed style、LayoutEpoch、terminal `1`をbindした。public/basic flow vocabularyは増やさず、precomposed-vector authorization専用のcrate-private parent projectionだけが既存Figure/DisplayMath categoryをreuseする。
  - validated source preorderを最初に全走査して0-based dense ID registrationを完了し、その後にshape/workを開始する二段階構造にした。worker completionを逆順にした場合もsource-order registryとfingerprintが一致し、missing/duplicate/non-dense/wrong owner・parent・position・epoch・terminal・shapeを再構築検証で`I9190`にする。terminal ledgerはpage moveで未消費を維持し、selected成功時のexactly-once consume、任意consume順からflow-ID順receipt set、missing/duplicate/wrong-owner/tamper拒否を保証する。
  - producerのequation-number TextSpanをexact sliceのまま、既存Unicode 16 itemizer、selected-face coverage検査、linked HarfRust backend、output budget、cluster validator、fixed-point position pipelineへ通し、one nonwrapping `typaxis.equation-number-shape/1` receiptを発行する。receiptはSourceSpan/TextSpan/buffer・slice hash、number text、computed style、font family/face/hash/index/size/line-height、shaper ID/version、Unicode version、glyph runs/receipt、positive width/heightをbindし、coverage fallback、text生成・increment・normalize、second lineを行わない。
  - equation numberはformula vector/source/alternative receiptと分離し、nonnullだけがV04で検証済みのowner+1 NodeIdを使う一つのleaf shapeになる。nullはshape/childを一切作らない。owner languageはwire inheritanceを一度だけsource tree traversalして作るsealed per-vector receiptを参照し、numberを独立language ownerにしていない。このnarrow receiptはMI4-V14のpublic computed-language registry `/2`を先取りしない。
  - native/vector交互fixtureをsource span順とdense global NodeId順で固定し、両flow ID空間が独立denseであること、number null/present、worker/page順 permutation、terminal closureを検査した。既存native `typaxis.math-flow/1` flow fingerprint `b075066e3ea4d2e9e0084dd4fbb9fa25f852a77c007ef655301d85c1fece4715`とlayout JCS SHA-256 `fae13b212b81b81a0e0ac38bce2943830e2f058b323ee5ec9d635bdcd630a8ab`をfrozen regressionにし、nominal ID swapはcompile-fail doctestで固定した。
  - milestone指定の3 targeted command、layout doctest、dependency-boundary test、Schema validator、workspace全target/all-feature check/test、workspace clippy `-D warnings`、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは3934 refsを含む全bundle/fixtureを通過した。
  - レビューではpublic `/1` profile guardを崩すparent projection、IDを全件登録する前のshape開始、交互fixtureの重複SourceSpanと番号不一致alt、owner-languageのper-owner再走査、terminal finishのunchecked sentinel/order closure、fallible ledger allocation、registry owner順とshape-style relationの局所検証不足を修正した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外では専用implementation module `workspace/crates/typaxis-layout/src/math_vector.rs`、equation-number font/交互fixture、sealed effective-language receipt、workspace dependency edge、本completion recordを変更した。blockのphysical placement/pagination、number rectangle/paint/structure、Display/PDF serialization、public capabilityはMI4-V11以降へ残した。
- Non-goals:
  - blockのphysical page placement、PDF text serialization

### MI4-V11 Block vector placement、equation number、atomic paginationを実装する

- Status: Completed
- Depends on: MI4-V10
- Design inputs: docs/27 §4.4、§5、§7、§15.2
- Primary files:
  - `workspace/crates/typaxis-layout/src/safe_vector.rs`
  - `workspace/crates/typaxis-layout/src/math.rs`
  - `workspace/crates/typaxis-layout/src/semantic_container.rs`
  - `workspace/crates/typaxis-layout/src/lib.rs`
  - `workspace/crates/typaxis-pagination/src/`
  - `schemas/1.4/layout-trace.schema.json`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - `math_vector_block` / `vector_figure`のselected placement。
  - independent equation-number rectangleとcontent-height proof。
  - style/keep/page/captionを含むatomic pagination closure。
- Tasks:
  1. computed styleからspace/indent/alignment/page/keepを確定し、checked inner frame widthを求める。formula viewport width/heightをproducer metricsから変更しない。
  2. `start|center|end`をhorizontal LTRのleft/center/rightへmapし、formula viewportはnumberの有無にかかわらずinner frame全幅に対して配置する。
  3. numberありでは`Bh = max(Vh, Nh)`、各child topを`round_half_even((Bh - child_height) / 2)`で求め、odd unitをblock-end側へ置く。numberなしでは`Bh = Vh`としnumber factを生成しない。
  4. equation numberをinner frame logical endへ置き、formulaとの間にpositive minimum gapを要求する。rectangle overlap、width overflow、nonpositive shapeを`L5100`にし、formula移動/縮小/wrapへfallbackしない。
  5. block viewport topからbaseline/pen originを導出し、viewport、pen origin、baseline、single scale、matrixを同じselected receiptへbindする。
  6. existing block spacing ownerでspace_before/after、page-top suppression、pending glue、page value、keep_with_nextを処理し、`effective_space_before + Bh`がfitしない場合はblock全体を次frameへ送る。
  7. empty full frameでもheight/widthがfitしない場合はNodeId/SourceSpan付き`L5100`にする。empty fragment、第二fragment、SVG内部split、clip successを作らない。
  8. `vector_figure`はexisting Figure caption flow、keep_caption、paint/structure source orderをreuseし、raster Figureとtyped media/placementを混同しない。
  9. selected block occurrenceを`max_fragments`へ一回だけ課金し、flow terminal、parent position、page/frame/block/paint ordinal、formula/number boundsをtraceへbindする。
  10. start/center/end、number null/short/tall/collision、page-end move、empty-page oversize、keep/page break、caption、native/vector math交互をfixture化する。
- Acceptance criteria:
  - SVG内部を一度も分割せず、selected blockは0または1 fragmentだけである。
  - paint/pagination/structure boundsが同じ`Bh`とformula/number rectangleを参照する。
  - page末でfitしないblockはwhole blockとして次pageへ移動し、empty pageでもfitしなければterminal errorになる。
  - number paintはformulaとは別rectangle/ownerで、reading order用のformula-first順を保持する。
  - existing Figure/native display-math pagination regressionが通る。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout math_vector_block_layout --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout vector_figure_layout --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination atomic_vector_block --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V11 change set containing this completion record.
  - `typaxis.precomposed-vector-block-preparation/1`でvalidated binding、private parent flow、`MathVectorFlowId`、computed vector style、LayoutEpoch、one-master page geometryを再照合してから、source-orderの`vector_figure` / `math_vector_block`だけをprepared blockへ閉じた。start/end indentからpositive inner frameをchecked計算し、horizontal LTRのstart/center/endをfull inner frameへ適用する一方、producer viewport、uniform scale、origin、baselineは変更していない。viewport width超過、unsupported named page、checked arithmetic failureはNodeId/SourceSpan付き`L5100`でfail closedにする。
  - numbered mathは既存nonwrapping equation-number shape receiptをjoinし、番号をlogical endへ独立配置する。`Bh = max(Vh, Nh)`、両childのhalf-even top offset、positive minimum gap、formula-firstのpaint/structure orderを同じprepared/selected closureへbindした。collision、number/frame overflowではformulaの移動・縮小・wrapへfallbackせず`L5100`にし、null番号ではrectangle、paint、structure childを一切作らない。
  - `typaxis.precomposed-vector-layout/1`のatomic block paginatorを追加し、page-top space suppression、pending `space_after + space_before`、forced/named page boundary、hard `keep_with_next` chain、Figure `keep_caption`、caption subflow、empty-page oversizeを処理する。SVG viewportは常に一つのfragmentとしてwhole moveし、各selected occurrenceをcumulative `max_fragments`へ一回だけ課金する。selected receiptはpreparationとpagination input、page/frame/block/paint ordinal、同一pagination/paint/structure bounds、viewport scale/matrix、pen origin/baseline、independent number、caption、parent position、exactly-once math terminalを再構築可能に保持する。
  - canonical `block-layout-trace.json`とprivate 1.4 Schemaを追加し、layout variantの相互排他、pagination input/page/placement、kind別Formula/Figure/number/caption child orderをclosed shapeにした。validatorはcanonical JCS、input/page/placement fingerprint、dense fragment/page/paint ordinal、bounds/matrix/baseline、number gap/source order、caption/page accounting、およびinvalid mixed trace・wrong child role・unnumbered number childの拒否を独立検査する。
  - start/center/end、number null/short/tall/collision、positive inner-frame width overflow、page-end whole move、empty-page height oversize、pending spacing、forced/named page、keep chain、kept/splittable caption、native/vector math交互、fragment exact/max+1、foreign preparation inputをunit/fixture testで固定した。milestone指定の4 command、workspace all-target/all-feature check/test、workspace clippy `-D warnings`、doc-test、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは4022 refsを含む全bundle/fixtureを通過した。
  - レビューではpagination inputが別prepared layoutへ流用できるclosure不足、keep chain末尾の`keep_caption = false` captionまで過剰にkeepする計算、caption/parent collectionのfallible allocation不足、unchecked index conversion、別pageへ送ったcaptionをowner blockのpageへ誤集計できるtrace検査、schemaのkind別structure order不足、width/caption-split acceptance fixture不足を修正した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外ではprepared block専用`workspace/crates/typaxis-layout/src/block_vector.rs`、atomic pagination専用`workspace/crates/typaxis-pagination/src/atomic_vector.rs`、fixture validator、本completion recordを変更した。既存native math/Figure algorithm bytes、Display/PDF/Form、language/accessibility、manifest/capability、public CLI integrationは変更せず、MI4-V12以降を先取りしていない。
- Non-goals:
  - shrink-to-fit、multi-line equation number、vertical writing

### MI4-V12 DrawVector Display `/2`とselected occurrence closureを実装する

- Status: Completed
- Depends on: MI4-V07, MI4-V09, MI4-V11
- Design inputs: docs/27 §5、§9.1、§10、§11
- Primary files:
  - `workspace/crates/typaxis-display-list/src/safe_vector.rs`
  - `workspace/crates/typaxis-display-list/src/math.rs`
  - `workspace/crates/typaxis-display-list/src/lib.rs`
  - `workspace/crates/typaxis-layout/src/safe_vector.rs`
  - `schemas/1.4/display-list.schema.json`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - `typaxis.draw-vector-display/2` command/receipt。
  - all 4 kindとexisting Figure usageのlogical resource closure。
  - deterministic usage/paint orderとtamper detection。
- Tasks:
  1. version-2 DrawVector commandへusage ID、owner NodeId/kind、image ID、VectorContentKey、IR fingerprint、selected placement fingerprint、page/frame/fragment/paint ordinal、viewport rectangle、single scale/matrix、resolved currentColorをbindする。
  2. inline/math usageだけにpen origin/baseline/metric receipt、math blockだけにMathVectorFlowId/terminal、generic Figureだけにcaption relationをconditional typed variantとして持たせる。
  3. commandはraw URI/SVG/TeX、PDF object/name/MCIDを持たず、source/alternativeはbinding fingerprintからjoinする。
  4. selected inline/block occurrenceとDisplay commandを双方向1:1に照合し、missing/extra/duplicate/wrong owner/kind/image/key/page/matrix/orderを`I9190`にする。
  5. page/paint ordinalからcanonical command orderとdense usage IDを発行し、worker completionやresource key順をpaint orderに使わない。
  6. Form finalizer `/2`がDisplay receiptだけからall usageをrecoverでき、zero-use resourceはcommandを持たないことを検証する。
  7. currentColor resolved paintをplacement factへ保持するが、content key/Form dedupe fingerprintへ混ぜない。
  8. existing `StagingDrawVector` / `typaxis.draw-vector-display/1`を変えず、version swap/tamperを拒否する。
  9. canonical JSON/Schemaへkind別conditional memberを実装し、permutation/tamper/old-golden fixtureを追加する。
- Acceptance criteria:
  - selected vector occurrence数とDisplay command数が一致し、各commandが一つのcontent key planへjoinできる。
  - same Formを異なるcolor/page/kindから使ってもcommandは別usage、content keyは同一になる。
  - baseline、viewport、matrix、page/paint orderのどれかを変更したrecordを検出する。
  - Display `/1` canonical bytesとexisting Figure PDF inputsは変更前と一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list draw_vector_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list precomposed_vector_display_tamper --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list draw_vector_v1_frozen --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V12 change set containing this completion record.
  - `typaxis.draw-vector-display/2`を専用moduleへ追加し、selected inline/block occurrenceからusage ID、owner/kind/image、exact `VectorContentKey`、IR/binding/selected-placement fingerprint、page/frame/fragment/paint ordinal、viewport、uniform scale/matrix、resolved currentColorを持つclosed DrawVector commandを発行する。inline二kindはmetric receipt・pen origin・baseline、`math_vector_block`はさらに`MathVectorFlowId`、flow fingerprint、parent FlowId/position、terminal `1`とterminal receipt、`vector_figure`はexisting caption flow/owners/keep relationだけをtyped conditional variantへ保持する。URI、raw SVG/TeX、alternative、PDF object/name、MCIDは型にもcanonical recordにも持たせていない。
  - package/profile/limits/admission/binding/LayoutEpoch/page geometryを共有するinline selected、block preparation、math-flow registry、pagination input、block selectedを再検証し、全binding ownerと全commandをowner順に双方向1:1で照合してから、selected `(page_index, paint_ordinal)`順へcanonicalizeしdense usage IDを割り当てる。missing/extra/duplicate/wrong owner/kind/image/key/IR/selected fingerprint/page/viewport/matrix/orderはsealed closureまたはupstream再構築照合で`I9190`にする。worker collection順を逆転しても同一receiptになり、command数とselected occurrence数、distinct selected content-key数をreceiptへbindする。
  - Display objectはpage/command wrapper fingerprintと全usageを保持し、後続Form `/2`がsource、SVG bytes、layoutを再読せずにimage alias、content key、page、matrix、resolved paintを復元できるresource-closure検証APIを持つ。fixtureは二ページ・全4 kindを一つのcontent keyで共有し、別内容のunused admitted vectorにはcommandを発行しない。異なるcolor/page/kindでもkey countは一つのままusageだけが別になることを固定した。content keyのnominal typeはdependency cycleを作らずDisplayとForm planningで共有するためadmission ownerへ移し、`typaxis-resources`から同じ型をre-exportする。raw tuple constructorは追加していない。
  - private contract-1.4 Display Schemaとcanonical `display-v2.json`を追加し、legacy Display shapeとのroot-level相互排他、algorithm `/2`、Safe-SVG media/parser/IR組、kind別baseline/caption/math-flow member、closed recordを固定した。validatorはJCS、page/command fingerprint、count、dense usage、unique owner/selected fingerprint、page/paint order、viewport/matrix、baseline、IR/content-key、four-kind/shared-key closureを検証し、version swap、mixed shape、raw TeX、wrong conditional member、baseline/viewport/matrix/page/order tamperを拒否する。existing `/1` receiptはcanonical SHA-256 `d844df26a1b70890b495141d2a67b270f0dd98ec436bc09d570196f9d23553f0`で凍結した。
  - milestone指定の3 targeted command、Schema validator、changed crate test、workspace all-target/all-feature check/test、workspace doc-test、workspace clippy `-D warnings`、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。Schema validatorは4071 refsを含む全bundle/fixtureを通過した。
  - レビューではselected/binding照合とduplicate検査の`O(n^2)`走査、math block commandから欠落していたparent FlowId/position、same-IR/different-source testがsource hash差を実admissionで証明しなくなっていた退行、private `/2` traceへlegacy root-page ruleを誤適用するvalidator分岐、distinct-key集計の非fallible allocation、unused resource/wrong-key substitutionのtest不足を修正した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外では専用implementation module `workspace/crates/typaxis-display-list/src/precomposed_vector.rs`、共有nominal key owner `workspace/crates/typaxis-resource-admission/src/lib.rs`と`typaxis-resources` re-export、fixture validator、README、本completion recordを変更した。PDF content stream/Form object/ExtGState/`Do`、MCID/structure/accessibility、manifest/public capability/CLI integrationはMI4-V13以降へ残した。
- Non-goals:
  - PDF content stream、MCID割当

### MI4-V13 PDF Form、ExtGState、placement `Do` closureを実装する

- Status: Completed
- Depends on: MI4-V07, MI4-V12
- Design inputs: docs/27 §8.2〜8.3、§9、§11、§15.3
- Primary files:
  - `workspace/crates/typaxis-resources/src/safe_vector.rs`
  - `workspace/crates/typaxis-pdf/src/safe_vector.rs`
  - `workspace/crates/typaxis-pdf/src/lib.rs`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
  - `tools/verify_pdf_differential.py`
  - `tools/test_pdf_differential.py`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - SafeVector Form plan `/2`、PDF relative-object-role/use contribution、final closure API。
  - vector-only Form XObject、Form-local ExtGState、page-local `Do`。
  - content-key順relative object-role/resource-name planningとindependent vector assertions。
- Tasks:
  1. DrawVector `/2` usageをimage IDからcontent candidateへjoinし、alias別/total usage countとselected paint-order usageを持つ`typaxis.safe-vector-form-plan(s)/2`を発行する。zero-use candidateはaudit inputへ残すがForm planを作らない。
  2. version-2 Form plan順でrelative object rolesとresource namesをallocateし、dedupe後のForm/ExtGState/page contribution count deltaをchecked計算する。absolute object numberを発行せず、`max_pdf_objects`もconsumeしない。first-use page/orderから割り当てない。
  3. Form `/BBox`をadmitted intrinsic viewport/viewBox mappingから作り、path、fill、stroke width/cap/join/miter、clipをPDF vector operatorへserializeする。image raster XObjectを生成しない。
  4. each drawを`q ... Q`で隔離し、FixedRgb8だけがcolor operatorを出す。CurrentColor drawはplacementから設定されたambient stroking/nonstroking colorを保持する。
  5. alpha pairごとのForm-local ExtGState dictionaryをnumeric orderで作り、`/Type /ExtGState`、`/ca`、`/CA`だけを出す。1/1もeach drawがexplicit `gs`で選択する。
  6. page usageは`q`、resolved RGB stroking/nonstroking、top-left placement matrix、`Do`、`Q`の順にする。docs/24 page-root Y flipを一度だけ適用し、viewBox/node/pageで二重flipしない。
  7. one content key = one Form object、N selected usages = N page-level `Do`をverifyし、zero-use keyにはobject/name/Doを作らない。
  8. reusable writer contributionへDisplay、Form plan、relative object role/resource name、usage/page/matrix、content-stream fingerprintをbindする。production `typaxis.safe-vector-pdf-closure/2`をsealするAPIはfinal writer bytes/hash/object tableをrequiredとし、`MI4-V16`より前にstandalone fixture hashをproduction receiptとして発行しない。
  9. Form streamへMCID/Alt/ActualText/Langを入れず、後続tagging ownerがpage-level `Do`をwrapできるsemantic usage hookだけを渡す。
  10. Form contributionのspool allocationをexisting ownerでmax+1前にconsumeし、partial contribution/PDF publicationを行わない。global object/output limitはcomplete final writer ownerの`MI4-V16`へ委ねる。
  11. isolated test writerとindependent parserでForm subtype/BBox/operator/clip/stroke/alpha/matrix、no raster、1 Form + 10 Do、200%/800%相当renderを検査する。public end-to-endのCurrentColorは現行style契約どおりexact blackで検査し、different-color shared Formはsealed paint planを注入するowner-private unit testだけで検査する。このtestのためにpublic color style propertyを追加しない。
  12. version-1 SafeVector PDF closure/goldenをbyte比較し、`/1` Form `/Resources << >>`等の意味を変更しない。
- Acceptance criteria:
  - 数式輪郭がForm path operatorで保持され、fixed pixel imageを参照しない。
  - currentColor/alphaがpreceding page/Form drawから漏れず、placement間でFormを共有できる。
  - `/BBox`、viewBox、single scale、page-root transformがlayout matrixとexact一致する。
  - Form/relative-object-role/useのmissing/extra/wrong key/name/orderをwriter-contribution verificationが拒否する。
  - 同一input/selected paint orderを保ったcandidate/worker order permutationでrelative object planとcontent stream bytesが一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf safe_vector_pdf_contribution_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf safe_vector_current_color --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf safe_vector_ext_gstate --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf safe_vector_pdf_v1_frozen --locked`
  - `python3 -m unittest tools/test_pdf_differential.py -v`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V13 change set containing this completion record.
  - `typaxis.safe-vector-form-plan/2`と`typaxis.safe-vector-form-plans/2`を追加し、sealed DrawVector `/2`をadmitted content candidateへ再joinする。各selected content keyについてnumeric image-ID順のalias別count（selected key内のzero-use aliasを含む）、total count、selected page/paint順usageを保持し、全candidate/aliasはregistry fingerprintとaudit countへ残す。zero-use content keyにはForm plan、relative role、resource name、`Do`を発行しない。Formはcontent-key順`V0...`、各FormのExtGStateはalpha pair順`GS0...`、relative roleはFormから始まるdense checked orderとし、absolute object numberと`max_pdf_objects` chargeを持たせていない。
  - `typaxis.safe-vector-pdf-contribution/2`を追加し、admitted canonical IRだけからintrinsic `/BBox`、root viewport clip、viewBox transform、path、quadratic-to-cubic、fill/even-odd、stroke width/cap/join/miter、local clipをPDF vector operatorへ変換する。each drawは`q`/`Q`で隔離し、alpha 1/1を含むresolved pairをexact minimal `/Type /ExtGState /ca /CA` dictionaryとexplicit `gs`へ閉じる。FixedRgb8だけがForm color operatorを出し、CurrentColorはForm内でambient colorを維持する。raster XObjectおよびMCID/Alt/ActualText/LangをFormへ入れない。
  - page contributionはselected paint orderで、各usageを`q`、resolved RGB8のnonstroking/stroking color、exact top-left uniform matrix、content-key Form名の`Do`、`Q`へserializeする。既存page ownerが一度だけroot Y flipを置くcoordinate policyをpage receiptへbindし、同一page/content keyのresource bindingだけをdedupeする。one content keyを10回使うfixtureは1 Form、1 page binding、10 `Do`となり、CurrentColorのpublic pathはexact black、異なる色で同じFormを再利用する性質はowner-private testで固定した。
  - contributionはDisplay/Form-plan/candidate/limits、relative object role/name、Form/ExtGState bytes fingerprint、page resource/use、matrix、resolved color、semantic usage hook、exact spool byte countをbindする。spool limit exact値は成功しmax+1開始前に失敗する。`typaxis.safe-vector-pdf-closure/2`はstandalone contributionから発行できず、non-Clone final PDF bytes receiptとcomplete writer由来のrelative-to-absolute object table、page/content/Form targetを含むexact usage observationを要求し、hash/length/page/object boundsと衝突を検査してからsealする。global object/output budgetとcomplete graph allocationはMI4-V16へ残した。
  - test-only isolated writerをfeature gate下に置き、classic dense PDFへForm-local ExtGState、page-local XObject mapping、one root flipを組み立てる。`typaxis-testkit`の独立parserはexact Form count/BBox/content hash、ExtGState count/alpha/name/target、vector path/clip/stroke operators、no raster/no Form semantics、MediaBoxに一致するroot flip、exact RGB/matrix/resource target、1 Form + 10 `Do`をfail-closedに検査する。Python differential gateも独立構造検査を持ち、144/576 DPI（200%/800%相当）を全pageでrenderできる。既定72 DPI digestの従来domainは維持する。
  - version-1 SafeVector writerへ変更を加えず、bytes長1213、SHA-256 `c7bb8e72adc0e60d303112647e978dd5d2db44fe82c56599644e23a68d61baff`、Form `/Resources << >>`、ExtGState absenceをbyte-frozen testで固定した。
  - milestone指定の5 command、changed crate tests、`cargo test --workspace --all-targets --all-features --locked`、workspace doc-test、workspace clippy `-D warnings`、fmt check、Schema validator、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。all-target/all-feature runのexplicit external-validator 2 testsは既存の`ignored`指定どおりであり、MI4-V13の独立Rust/Python検査は通常testとして成功した。
  - レビューではfinal object/page object衝突、page `Do`とabsolute Form targetの未結合、assertion-only writerと独立parserの未接続、Form stroke・ExtGState target・page resource/usage-to-Form/matrix/MediaBox/currentColor期待値の検査不足、非canonical/NaN alpha受理、同一key内zero-useを含む複数alias countと新`/2` Safe-SVG 1 branchのtest不足、巨大DPIの未整形overflow、既定72 DPI digest domainの意図しない変更を修正した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外ではversion-2専用module `workspace/crates/typaxis-resources/src/safe_vector_v2.rs`、`workspace/crates/typaxis-pdf/src/safe_vector_v2.rs`、独立parser `workspace/crates/typaxis-testkit/src/safe_vector_pdf.rs`、test-only Display projections、Cargo feature wiring、sample README、本completion recordを変更した。tagged structure、ActualText、complete final writer integration、manifest/public capability/CLI integrationはMI4-V14以降へ残した。
- Non-goals:
  - complete final graphのabsolute object number/global object-budget charge、tagged structure、ActualText、public capability

### MI4-V14 Computed languageとbook-navigation chain `/2`を実装する

- Status: Completed
- Depends on: MI4-V04, MI4-V09, MI4-V11
- Design inputs: docs/27 §3、§4.3〜4.4、§10、§11、§15.3
- Primary files:
  - `workspace/crates/typaxis-document/src/book_navigation.rs`
  - `workspace/crates/typaxis-syntax/src/book_navigation.rs`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/book_navigation.rs`
  - `workspace/crates/typaxis-display-list/src/book_navigation.rs`
  - `workspace/crates/typaxis-pdf/src/book_navigation.rs`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - `typaxis.computed-language-registry/2`。
  - book-navigation profile view/receipt/selected state `/2`。
  - final tagged PDFから`book-navigation-pdf/2`を発行するためのsealed input contract。
- Tasks:
  1. closed language owner kindへ`inline_vector`、`math_vector`、`vector_figure`、`math_vector_block`を追加したversion-2 enum/recordを作り、source NodeId preorderでeach owner exactly once登録する。
  2. existing BCP 47 parse/canonicalization、document/semantic-container/flow inheritanceをreuseし、Alt/resolved ActualTextへ適用するeffective languageをpackage/navigation/profile/limits fingerprintへbindする。
  3. equation numberを第五のlanguage ownerにせず、parent math-vector-blockのcomputed language fingerprintを参照するchild recordにする。vector_figure captionはexisting inheritance childとして扱う。
  4. missing/extra/duplicate/wrong-kind/wrong-parent/order/language、`/1` registry swapをsyntax/profile boundaryで拒否する。
  5. `typaxis.book-navigation-profile-view/2` / receipt `/2`へ完全なowner setをbindし、selected vector paintとcomputed languageを`typaxis.book-navigation-selected/2`へ1:1で関連付ける。
  6. selected stateにpage/paint/owner/languageを保持し、physical paint orderからlogical inheritanceを推測しない。splitしないvectorでもlogical owner orderをsource registryから使う。
  7. `typaxis.book-navigation-pdf/2` constructorはfinal PDF hash、Info、catalog `/Lang`、outline、language paint、unchanged `typaxis.book-xmp/2` observationを同時に要求する。実際のfinal tagged PDF observationは`MI4-V16`で発行する。
  8. same metadata/language inputに対するXMP bytesが従来の`typaxis.book-xmp/2`と一致し、version bumpをlanguage owner chainだけへ限定するtestを追加する。
  9. raw/canonical language text chargeを`/1`から引き継ぎ、`/2` registryへのprojectionでresetまたは二重加算しない。
  10. document language inheritanceとexplicit overrideを4 kindすべてでpositive fixture化し、missing/extra/tamper/old-profile rejectionを追加する。
- Acceptance criteria:
  - 4 kindがNodeId順にexactly once computed-language `/2`へ現れる。
  - selected paintとstructureで使用するlanguage fingerprintが同一ownerのregistry recordへjoinできる。
  - document languageと異なるplacementだけを後続paint-level Lang対象として判定できる。
  - metadata、outline、destination、XMP serialization identityの既存意味を変えない。
  - computed-language/book-navigation `/1` Schema/JCS/fingerprint/goldenが変更前と一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax computed_language_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile book_navigation_profile_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list book_navigation_selected_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf book_navigation_vector_input_v2 --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-03, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V14 change set containing this completion record.
  - `typaxis.computed-language-registry/2`を追加し、既存19 kindと4 vector kindをnominalなclosed owner enumへ収め、dense source-preorder NodeId順でexactly once登録する。各recordはpackage/semantic/base+M4 limits、existing BCP 47 canonicalization、logical parent、raw/canonical text charge、effective language、source span、per-record fingerprintをbindする。vector recordはV04のmetrics/effective-language receiptとAlt/resolved ActualText hashを再joinし、prevalidated authored-language chargeを総量へ一度だけ含める。equation numberは第五ownerにせずparent `math_vector_block` record fingerprintを参照するchild recordとし、vector Figure captionはFigureのeffective languageを既存flow inheritanceで受け継ぐ。
  - `typaxis.book-navigation-profile-view/2`、`typaxis.book-navigation-profile-receipt/2`、`typaxis.book-navigation-selected/2`を追加した。profile receiptはprecomposed-vector profile receipt/authorizationと23-kind descriptorを閉じ、selected receiptはnonzero selected-layout fingerprint、page/paint/owner/language、computed-language record fingerprint、Display command fingerprintを保持する。vector paintはphysical page/paint順に並べる一方、logical ordinalはcomputed-language registryから取得し、全vector ownerを1:1で閉じる。document languageと異なる4-kind placementだけを後続paint-level `/Lang`対象として列挙する。
  - `typaxis.book-navigation-pdf/2`のsealed input contractを追加し、同じ最終`VerifiedPdfBytesReceipt`のhash/length/page/object stateへInfo、catalog language、destination registry、outline hierarchy、必要なlanguage paint、`typaxis.book-xmp/2`を同時に結ぶ。navigation component hashをmetadata/language/outline/destination別に明示し、navigation/outline/page-content object collisionとpage-to-content mappingをfail closedにする。actual final tagged-PDF writerからこのobservationを発行する処理は予定どおりMI4-V16へ残した。
  - tagged-PDFの既存XMP encoder本体をmetadata/language引数の共有helperへ抽出しただけでserialization bodyは変更していない。同一metadata/languageをlegacy navigationと`/2` projectionから与えたbytes一致とSHA-256 `f2d02831c768180f5121517593f783331fc148ed59bafeb21f3d13da69dc3a5f`を固定した。既存book-navigation `/1`のfull manifest golden testも変更なしで成功し、computed-language/profile/selected/PDF fingerprintとcanonical bytesを維持した。
  - inheritance fixtureと、4 kindすべてへraw `EN-us`を指定してcanonical `en-US`を得るchecked-in override fixtureを追加した。layout/Displayのtest-only override caseも通常のprofile/admission/binding/layout経路から再生成し、receiptを書き換えずに4 paintすべてがlanguage対象となることをDisplay/PDF境界で検証する。missing/duplicate/wrong parent/kind/order/language、child fingerprint tamper、old `/1` vector path、wrong profile、zero layout fingerprint、missing/extra paint、catalog/XMP/object collisionを拒否するtestを追加した。
  - milestone指定の5 command、V1 book-navigation golden、`cargo test --workspace --all-targets --all-features --locked`、workspace doc-test、workspace clippy `-D warnings`、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。all-target/all-feature runのexternal-validator 2 testsは既存の`ignored`指定どおりであり、Schema validatorは4071 refsを含む全bundle/fixtureを通過した。
  - レビューではV1/V2 profile authorization引数の誤配置、selected `/2`のzero layout fingerprintとconstructor自己検証漏れ、computed-language text-limit診断、computed-language fingerprintをnavigation全体と誤称したPDF field、outline/navigation/page-content object collision、unchecked XMP length変換、vector Figure caption継承test不足、receiptだけを書き換える不正確なoverride fixtureを修正した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外ではtest-only language overrideのため`workspace/crates/typaxis-layout/src/block_vector.rs`、`workspace/crates/typaxis-layout/src/safe_vector.rs`、Display fixture module、既存XMP helper、V04 narrow-language receiptのstale comment、sample README、本completion recordを変更した。structure role/MCID/ActualText serialization、complete final tagged-PDF writer、manifest/public capability/CLI integrationはMI4-V15以降へ残した。
- Non-goals:
  - TeX dialect選択、equation-number固有language override
  - final tagged PDFのserialization

### MI4-V15 Formula/Figure structure registryとmarked-content plan `/2`を実装する

- Status: Completed
- Depends on: MI4-V12, MI4-V14, MI4-09
- Design inputs: docs/27 §3、§10、§15.3〜15.4
- Primary files:
  - `workspace/crates/typaxis-syntax/src/tagged_structure.rs`
  - `workspace/crates/typaxis-layout-contract/src/tagged_structure.rs`
  - `workspace/crates/typaxis-display-list/src/tagged_structure.rs`
  - `workspace/crates/typaxis-display-list/src/safe_vector.rs`
  - `workspace/crates/typaxis-machine-profile/src/tagged_pdf.rs`
  - `workspace/crates/typaxis-machine-profile/src/lib.rs`
- Deliverables:
  - structure-role vocabulary/registry/selected binding `/2`。
  - vector用outer MCR + inner property-only Spanを表すmarked-content plan `/2`。
  - equation-number structure childとForm-stream isolation proof。
- Tasks:
  1. version-2 semantic registryへ4 kindをexhaustiveに追加し、mathはFormula、generic vectorはFigureへsource owner preorderでbindする。mathをArtifactへ分類できない型にする。
  2. structure nodeへexact Alt、computed language fingerprint、source span、logical parent/child order、selected paint ownerをbindする。math resolved ActualTextはmarked-content childへ渡し、TeXを代用しない。
  3. `math_vector_block`にequation numberがある場合だけ、Formula `/K`のvector MCR後へsource-owned Span childを追加する。numberのTextSpan/glyph/language receiptをbindし、Formula ActualTextへ連結しない。
  4. selected DrawVector `/2` usageをouter Formula/Figure MCRへexactly once bindし、inner property-only Spanのrequired/optional matrixを§2.7どおり決める。
  5. mathはresolved ActualTextを持つinner Spanをrequired、inline vectorはnonnull authored ActualTextまたはpaint-level Langが必要なときだけ、vector_figureはLangが必要なときだけinner Spanを持つ。
  6. outer MCRだけにdense page-local MCIDを発行し、inner SpanはMCIDを持たない。Form streamはstructure occurrenceとしてcountせず、page-level `Do` usageだけをownerにする。
  7. role/parent/order/alternative/language/selected paintのmissing/extra/duplicate/swapと、Form内MCID injectionを`I9190`で拒否する。
  8. generated structure/depth/string/marked occurrence/MCIDをADR-0035のexisting limitsへissue前にone-time chargeし、`/2`移行でbudgetをresetしない。
  9. existing paragraph/list/table/Figure/native Formula/Link/footnote/container registryとのlogical orderを維持し、new kindだけをversion-2 vocabularyへ加える。
  10. vector kind全組合せ、number null/present、language equal/different、actual null/nonnull、wrong role/owner/order/MCID/versionをfixture化する。
- Acceptance criteria:
  - visual vector usageとouter Formula/Figure MCRのmissing/extraが0件である。
  - each pageのMCIDが0始まりdenseで、inner property SpanとForm streamはMCIDを持たない。
  - equation numberはformula paint後のlogical child/reading orderにexactly once現れる。
  - Alt、ActualText、Langのkind別presence/absenceが§2.3/§2.7 matrixと一致する。
  - structure/marked-content `/1` canonical bytesと既存native math/Figure fixtureは変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout-contract vector_structure_registry_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list vector_marked_content_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile accessibility_profile_v2 --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-09-04, macOS Darwin 25.5.0 arm64, rustc/cargo 1.97.1):
  - Implementation commit: this MI4-V15 change set containing this completion record.
  - syntaxとmachine-profileへversion-2 structure-semantic/profile-view projectionと、採択済みauthorization/accessibility-preflight `/2`を追加した。前二者は独立identityを作らず、それぞれ`typaxis.structure-registry/2`と`typaxis.production-accessibility-preflight/2`のcomponentとしてdomain-separateする。4 vector kindをcomputed-language `/2`へ再結合し、mathをFormula、generic vectorをFigure、equation numberをparent Formula直下のsource-owned Spanとして閉じた。Formula/FigureのAlt、kind別resolved/authored ActualText、source span、metric fingerprint、equation-number TextSpan/text hashを保持し、math ActualTextへTeXまたはnumber textを代用・連結しない。
  - layout-contractへ30-roleの`typaxis.structure-role-vocabulary/2`、`typaxis.structure-registry/2`、`typaxis.selected-structure-binding/2`を追加した。registryはsource/generated NodeId順、logical parent/child、nearest-parent language relation、4-kind role/metric matrix、optional equation childをcanonical JCSへbindする。selected bindingは各DrawVector usageをone Formula/Figure ownerへexactly once結び、equation-number paintを直前のmath-vector-block usage、通常text shape/glyph receipt、親子language fingerprintへ結ぶ。missing/extra/duplicate/wrong role・owner・metric・parent・order・language・versionは`I9190`、既存limit超過はowner codeでfail closedにする。
  - display-listへ`typaxis.marked-content-plan/2`を追加した。page paint順からreal groupだけへ0始まりdense MCIDを採番し、outer Formula/Figure MCRだけがMCIDを所有する。mathはActualText必須のproperty-only inner Span、generic inlineはauthored ActualTextまたはpaint-level Lang時だけ、vector Figureはpaint-level Lang時だけinner Spanを持つ。non-Spanの既存native Formula/Figure等もADR-0035どおりpropertiesをinner Spanへ置き、Span ownerはouter dictionaryを維持する。Formula logical `/K`はvector MCR、存在する場合だけequation-number Span childの順であり、number自身のtext MCRを別recordとして保持する。
  - Form isolation projectionはselected content keyごとのForm数、page-level `Do`数、zero Form MCID/structure-property countをDisplay receiptへbindし、独立identityを追加せず`typaxis.marked-content-plan/2`のcomponent proofとした。MI4-V13のForm encoder/parserによるno MCID/Alt/ActualText/Lang検査と合わせ、再利用Formをstructure occurrenceとして数えない。marked-content recordはselected-layout fragment countへ追加で一回だけ`max_fragments`をchargeし、allocation/MCID issue前にexact/max+1を判定する。AST/string/languageの既存chargeはprojectionでresetまたはaggregateへ再加算しない。
  - fixtureは4 kind、number present/null、document language equal/explicit override、inline actual null/nonnull matrix、native Formula/Span property scope、page-local dense MCID、Formula child order、Form injection、wrong role/Alt/language/parent/metric/owner/order/MCIDを検査する。version-1 semantic/registry/selected/marked-content encoder本体は変更せず、既存tagged-structure test、`draw_vector_v1_frozen_canonical_bytes`、`safe_vector_pdf_v1_frozen_bytes`を含むworkspace回帰で旧bytes/意味を維持した。
  - milestone指定の4 command、`cargo test --workspace --all-targets --all-features --locked`、workspace doc-test、workspace clippy `-D warnings`、fmt check、`/usr/bin/git diff --check`をlocalで実行し、すべてexit 0。all-target/all-feature runのexternal-validator 2 testsは既存の`ignored`指定どおりであり、Schema validatorは4071 refsを含む全bundle/fixtureを通過した。
  - レビューでは設計にないForm-isolationおよびsemantic/profile-view algorithm identity、selected bindingのlimit/allocation error潰し、paint数をfragment上限と誤解した二重制約、standard non-Span ownerのActualText/Langをouterへ置く誤り、native Formulaを欠落させるFormula order集合とphysical-order依存を修正した。empty captionが予約するnonpainting paint ordinal gapは保持し、MCIDだけを独立にdense化した。修正後に全差分を再読し、findingは0件である。
  - listed primary file外ではversion-2 typeのre-exportに`workspace/crates/typaxis-syntax/src/lib.rs`、`workspace/crates/typaxis-layout/src/lib.rs`、`workspace/crates/typaxis-display-list/src/lib.rs`を変更し、本completion recordを追加した。final PDF StructTree/ParentTree serialization、tagged-PDF observation、in-tree/external validator claim、manifest/public capability/CLI integrationはMI4-V16以降へ残した。
- Non-goals:
  - PDF StructTree serialization、veraPDF claim

### MI4-V16 Tagged PDF、book-navigation PDF observation、in-tree validator `/2`を実装する

- Status: Complete
- Depends on: MI4-V13, MI4-V15
- Design inputs: docs/27 §3、§9.1、§10、§11、§15.3
- Primary files:
  - `workspace/crates/typaxis-pdf/src/safe_vector.rs`
  - `workspace/crates/typaxis-pdf/src/book_navigation.rs`
  - `workspace/crates/typaxis-pdf/src/tagged_pdf.rs`
  - `workspace/crates/typaxis-pdf/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/tagged_pdf.rs`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
  - `tools/verify_pdf_structure.py`
  - `tools/test_pdf_structure.py`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
- Deliverables:
  - tagged PDF observation/validator `/2`。
  - page-level Formula/Figure MCR、ActualText/Lang Span、equation-number child。
  - same-final-hash `book-navigation-pdf/2` observation。
- Tasks:
  1. `typaxis.pdfua1-profile/2`、production preflight/authorization `/2`からだけvector tag serializationを許可し、old authorizationとのswapを拒否する。
  2. page contentでouter semantic BDC + MCID、inner property-only Span + resolved ActualText/conditional Lang、DrawVector `Do`、inner EMC、outer EMCの順をexactにserializeする。
  3. Figure kindはkind別ActualText absence規則を守り、language overrideだけ必要な場合はLang-only inner Spanを出す。AltはStructElem、ActualTextはmarked-contentへ分離する。
  4. Form XObject streamにMCID/Alt/ActualText/Langがないことをre-deriveして検査し、shared Formの各page usageだけに別MCID/semantic propertyを付ける。
  5. equation-number text MCR/SpanをFormula `/K`でvector MCRの直後に置き、normal text glyph/extraction receiptとparent computed languageを使う。
  6. V13のrelative vector object rolesをfont/image/page/metadata/structure等の全writer contributionとcomplete final indirect-object graphへmergeし、canonical role orderを確定する。全actual object countをchecked計算し、absolute number割当前に`max_pdf_objects`をexactly once consumeしてから全objectを一度だけ採番する。max+1は`G6100`でpartial object/PDFなしに拒否する。
  7. StructTreeRoot、RoleMap、StructElem、MCR、ParentTree、IDTree、page StructParentsをversion-2 registry/marked planだけからserializeし、object/MCID orderをselected planから固定する。complete final bytesは既存writer ownerで`max_output_bytes`を一回だけconsumeしてからatomic publicationへ渡す。
  8. final tagged PDF hashから`typaxis.tagged-pdf-observation/2`と`typaxis.book-navigation-pdf/2`を発行し、両者が同じPDF hash、catalog Lang、outline/XMP observationを参照することを検証する。
  9. same final bytes/hash/object tableで`typaxis.safe-vector-pdf-closure/2`をsealし、tagged/book-navigation observationsとPDF hashが一致しなければ拒否する。
  10. writer-independent in-tree validator `/2`へnew roles、inner property Span、equation child、shared Form no-MCID規則を追加し、leaf-type/closure tamperをfail closedにする。
  11. missing/wrong Alt/ActualText/Lang、wrong role/order/page/MCID/ParentTree、Form MCID injection、same-length stream tamper、duplicate/missing object charge、`/1` receipt swapをnegative testにする。
  12. independent text extractionでTeX tokenではなくresolved ActualTextが前後の日本語/句読点/numberとdocument orderで得られることを検査する。
  13. existing tagged-PDF `/1`、book-navigation `/1`、book-xmp `/2` golden bytesをbyte比較する。
- Acceptance criteria:
  - mathはFormula、generic vectorはFigureとして構造化され、Artifactへ落ちない。
  - Alt/ActualText/Langとequation number reading orderがindependent validator/extractorで一致する。
  - shared Formにsemantic stateがなく、same Formを使う各placementが個別MCRを持つ。
  - vectorを含むcomplete final graphのabsolute object numberと`max_pdf_objects` chargeはV16で一回だけ確定し、V07/V13のrelative countとのjoinがexactである。
  - book-navigation PDF `/2`とtagged observation `/2`が同一final PDF hashへ閉じる。
  - `/1` validator/manifest inputと`/2` observationを混在させると`I9190`になる。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf tagged_vector_pdf_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf complete_pdf_object_graph_budget --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf vector_actual_text_extraction --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf book_navigation_pdf_v2 --locked`
  - `python3 -m unittest tools/test_pdf_structure.py -v`
- Completion record:
  - `workspace/crates/typaxis-pdf/src/tagged_pdf_v2.rs`にproduction authorization/structure/marked-content `/2`を検証するstaging writerを追加した。PDF crateへlayout/pagination/machine-profileの直接依存を追加せず、Display-list所有のsealed borrowing projection `VectorMarkedContentSerializationV2`経由でselected geometryとequation-number shaping receiptを受け取る。projection自体のcanonical identityやresource chargeは増やさない。Rust独立validatorのpositive fixtureは実際のproduction profile preflight/session verificationを経由する。
  - vector relative object roles、equation fonts、page/content、navigation、Info/XMP、structure/ParentTree/conditional IDTreeを一つのfinal object planへ集約した。全actual countをchecked計算して`max_pdf_objects`を一回判定した後だけabsolute numberを発行し、exact/max+1、missing/duplicate role/chargeを検証する。final bytesは既存`VerifiedPdfBytesReceipt`で保持し、tagged observation `/2`、book-navigation PDF `/2`、SafeVector final closure `/2`を同一PDF hash/object tableへ閉じる。
  - Formula/Figureごとのouter BDC/MCID、kind別ActualText/conditional Langのproperty-only Span、shared Form `Do`、Formula直後のequation-number child Spanをserializeする。番号は既存shapeのglyph/position/source clusterを使い、resources側のstaging text finalizerでTrueType subset、Type0/CID、ToUnicode/CIDToGIDMapを決定的に生成する。font/CID resource limitは`G6100`を保持し、whole-number ActualTextにより多scalar/single-glyph clusterも元の番号文字列を抽出する。番号のためにTeXや文字列を再組版しない。
  - writerから独立した`typaxis-testkit/src/tagged_pdf_v2.rs`とPython validator `/2`でfinal PDF/object hash、xref/trailer、Catalog/Lang/XMP、outline destination/page/SE、IDTree、ParentTree、role/Alt/ActualText/Lang、page-local MCID/Formula child order、Form no-semantic-state、font resource/CIDと抽出順を検査する。missing/wrong property、same-length stream tamper、Form BMC injection、bad destination/parent cycle、old observation algorithm、非一回charge、wrong object role、CMap operator/count/CIDをnegative testにした。
  - positive fixtureは四vector kindを二page上の一Form/four Doとして再利用し、captionless blockと番号、document language override、nonempty title/outlineを含む。日本語resolved ActualText、句読点、番号のdocument order、TeX token不出力、Unicode番号「第1式」、ActualText-onlyの空ToUnicode mappingを検証する。optional IDTreeは独立synthetic positive/negativeで双方向参照を検査する。VMB全categoryと前後の通常本文を合わせたcombined book pipelineはMI4-V18、external validator/release claimはMI4-V19のままとする。
  - 指定のPDF test四filter、`cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit tagged_pdf_v2 --locked`、Python structure suite（28 tests）、workspace all-target/all-feature locked tests、doc-tests、Clippy `-D warnings`、fmt check、Schema validator（4071 refs）、`/usr/bin/git diff --check`をlocalで実行した。既存external-validator二testは従来どおりignoredであり、外部適合性のsuccessには数えない。既存tagged-PDF `/1`、book-navigation `/1`、book-XMP `/2`の実装/fixture bytesを変更せずworkspace回帰で確認した。
  - レビューでunembedded番号font、依存firewall違反、outline/IDTreeとtrailer closureの不足、Unicode番号のASCII-only制約、resource limitの`I9190`への変換、ActualText-only fontの不当拒否を修正した。修正後の全差分レビューのfindingは0件。追加primary外変更はDisplay-listのprojection/re-export、layoutのtest-only fixture case、resourcesのtext finalizer/re-export、testkitの独立validator module/dev-dependency、Cargo manifest/lockと本recordに限定し、public capability/CLI、manifest `/2`は変更していない。
- Non-goals:
  - external validatorによるrelease claim。`MI4-V19`で閉じる

### MI4-V17 SafeVector/math-vector/build manifestとcapability stagingを閉じる

- Status: Pending
- Depends on: MI4-V07, MI4-V13, MI4-V14, MI4-V16
- Design inputs: docs/27 §3、§11、§12、§13
- Primary files:
  - `workspace/crates/typaxis-manifest/src/safe_vector.rs`
  - `workspace/crates/typaxis-manifest/src/math_vector.rs`
  - `workspace/crates/typaxis-manifest/src/book_navigation.rs`
  - `workspace/crates/typaxis-manifest/src/tagged_pdf.rs`
  - `workspace/crates/typaxis-manifest/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/descriptor.rs`
  - `workspace/crates/typaxis-machine-profile/src/capabilities.rs`
  - `workspace/crates/typaxis-machine-profile/src/lib.rs`
  - `schemas/1.4/build-manifest.schema.json`
  - `schemas/1.4/machine-safe-vector-manifest.schema.json`
  - `schemas/1.4/machine-math-vector-manifest.schema.json`
  - `schemas/1.4/machine-book-navigation-manifest.schema.json`
  - `schemas/1.4/machine-accessibility-manifest.schema.json`
  - `schemas/1.4/machine-capabilities.schema.json`
- Deliverables:
  - SafeVector manifest `/2`、math-vector manifest `/1`、book-navigation manifest `/2`、tagged-PDF manifest `/2`。
  - production build-manifestのvector chain closure。
  - exact private capability projectionとpublic isolation。
- Tasks:
  1. SafeVector `/2`へcontent-key順resource fact、numeric image-ID alias、conditional provenance、parser/IR/allocation/intrinsic facts、V16 complete-final-graph receipt由来のconditional absolute Form object/resource name、total/alias placement count、paint-order usage/matrixを記録する。V13 relative planからabsolute numberを推測しない。
  2. math-vector `/1`へmath kindだけのNodeId/SourceSpan、source-TeX TextSpan ID/start/end、TextBuffer hash、exact slice hash、Alt/resolved ActualText hash、language、全metric、spacing/style/number、flow/terminal、binding/selected/usage fingerprintを記録する。generic vector factにmath referenceを置かない。
  3. tagged manifest `/2`はSafeVector manifest/usage fingerprintとtop-level math-vector fingerprintを参照し、each math structure factだけが対応math binding fingerprintを参照する。SafeVector -> math-vector -> taggedのacyclic方向を守る。
  4. book-navigation manifest `/2`はcomputed-language/profile/selected/PDF observation `/2`を参照し、tagged manifestから逆参照しない。
  5. production built branchのexisting book-navigation record/fingerprint pairを`/2`へ置換したうえで、SafeVector `/2`、math-vector `/1`、tagged `/2`のrecord/fingerprint pairを三組required nonnullとして追加する。existing native math manifest `/1` pairは置換しない。
  6. failed branchではeach pairをrequired nullable、both-nullまたはboth-nonnullに限定し、complete owner到達前のempty/synthetic recordを作らない。builtでzero resource/usageでもcanonical empty recordをnonnullで出す。
  7. kind別inapplicable fieldsをSchema conditionalでforbidし、resource/placement/alias/usage順とall identity/versionをcanonical JCSへ固定する。
  8. private production descriptorへ設計§12のcomplete merged `blocks`、`inlines.kinds`、`style_block_types`、`style_selectors`、coarse `image_formats = [jpeg, png, svg]`、`vector_formats`、`vector_profiles`、`vector_metrics`、`vector_features`、`vector_features_by_profile`、`vector_media_by_kind`、resource-set `/2`、component/media fixed orderを登録する。`svg-safe-1|2`を`image_formats`へ入れない。
  9. set-valued vector arrays/value arraysをUTF-8 byte順、JSON object keysをJCS UTF-16順、resource component/media arraysをADR fixed orderにする。profile tupleの将来位置はparagraph後/table前、defaultはparagraphのままとする。
  10. capability projectionとpreflight accepted setを双方向testしつつ、public serializerは七profile/current 1.3 bytesを出し続け、private profileをadvertiseしない。
  11. production resource-set `/2`では`svg-safe-1`を使うexisting FigureもSafeVector manifest `/2`へprojectし、同じbuild内へSafeVector manifest `/1`を混在させない。
  12. missing/extra/wrong fingerprint/order/count/kind/conditional field/version swap、built/failed/zero-use/unused aliasをtamper fixture化する。
- Acceptance criteria:
  - root build manifestからresource、math binding、language、structure、PDF useをfingerprintで一方向joinできる。
  - engine/version/rules、SVG hash、metric、placement count、parser/IR/layout/dedupe identitiesがaudit viewから欠落しない。
  - same content aliasはone Form factと複数alias provenanceを持つ。
  - manifestのabsolute Form objectはV16 final object tableと一致し、relative planや別phaseで再採番されない。
  - built/failed/zero-useのSchema semanticsが曖昧でなく、missing phaseをempty recordで偽装しない。
  - public `capabilities --format json`、public capability Schema/fixture bytesは変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-manifest safe_vector_manifest_v2 --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-manifest math_vector_manifest --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-manifest vector_build_manifest_closure --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile precomposed_vector_capability_projection --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile public_capability_isolation --locked`
  - `python3 schemas/validate.py`
- Non-goals:
  - current Schema alias、public profile tupleの切替

### MI4-V18 Private CLI integration、combined fixture、negative/reproducibility gateを閉じる

- Status: Pending
- Depends on: MI4-V17
- Design inputs: docs/27 §15、§16 steps 8〜9
- Primary files:
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `workspace/crates/typaxis-cli/src/machine_tests.rs`
  - `workspace/crates/typaxis-cli/tests/cli_end_to_end.rs`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
  - `schemas/1.4/machine-fixture-expectation.schema.json`
  - `schemas/1.4/machine-precomposed-vector-evidence.schema.json`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
  - `tools/verify_precomposed_vector.py`
  - `tools/test_precomposed_vector.py`
  - `tools/verify_reproducibility.py`
- Deliverables:
  - crate-private 1.4 production pipelineのend-to-end vector closure。
  - VMB combined、negative/tamper、two-build/path-alias fixture evidence。
  - public command/profile rejection regression。
- Tasks:
  1. existing crate-private production runnerへWire -> syntax metrics/source/computed-language -> profile/style -> resource -> metric/math binding -> inline/block layout -> Display `/2` + selected navigation -> content/Form plan -> structure/marked-content plan -> final tagged PDF + vector/navigation/tagged observations -> manifestsのstrict phase順を接続する。
  2. preflight拒否ではresource open/layout/PDF tempを開始せず、resource admission failureではlayout以降を開始せず、late failureでは既存failed-manifest/publication policyへcomplete receiptだけを投影する。
  3. corpus全categoryを含むJapanese combined documentを作り、source/package/resource hash、page count、normalized extracted text、placement/resource/Form/Do counts、language/structure factsをexpected ledgerへ固定する。
  4. inline baseline、advance width、line spacing suppression、Japanese boundaries、max ascent/descent、block alignment/number、page-end move、same-content dedupeをtrace/manifest/PDFから三方向に検査する。
  5. malformed/forbidden SVG、hash/provenance、metrics、alternative/span、style、flow、language、structure、content-key/Form/use/manifest tamper、width/height/object/fragment/text/AST/vector limit max+1をindividual invalid fixtureへする。
  6. each negative caseにexpected phase/code/location、visible artifacts、resource read/PDF temp有無を記録し、silent omissionまたはsuccessを禁止する。
  7. `verify_precomposed_vector.py`をgenerated artifact path入力のindependent parser/extractorとして実装し、sample directoryへ生成物を書き戻さない。同toolにper-host canonical evidenceを発行する`--emit-host-evidence`と、複数hostを集約する`--require-host-evidence` modeをprivate `machine-precomposed-vector-evidence` Schema付きで実装する。
  8. identical package/resource bytes、dense IDs、selected paint orderで二回buildし、owner-private candidate列挙順またはworker completion scheduleだけを変えてPDFと全sidecar bytesを比較する。resource declaration順の変更は別入力なのでsame-input determinism testに混ぜない。
  9. `verify_reproducibility.py`の異名checkout modeへprivate staging test entryを追加し、source path、locale、timezone、filesystem orderに依存しないことを検査する。public CLIへhidden staging optionを追加しない。
  10. public CLI E2Eでcontract 1.4/new kind/new media/private profileを引き続き拒否し、help/current constants/capabilities/default/Schema aliasのgoldenを比較する。
  11. `/1` SafeVector/native math/book-navigation/tagged fixturesを同じtest runで再検証する。
- Acceptance criteria:
  - 設計§15.1〜15.5の全assertionがfixture/test IDへtraceできる。
  - combined PDFはsilent deletion、rasterization、TeX extraction、vector splitを持たない。
  - same SVG hashの10 placementが1 Form + 10 Do、cross-ID aliasが1 Form + 2 provenance factになる。
  - 全negative caseがterminal failureとなり、phaseに応じたside-effect policyを守る。
  - same input/binary/font/resourceのPDFと全sidecarがtwo-buildおよび異名checkoutでbyte一致する。
  - public current surfaceは変更前のgolden bytesと一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_precomposed_vector --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_precomposed_vector_negative --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_precomposed_vector_reproducibility --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli public_m4_vector_isolation --locked`
  - `python3 -m unittest tools/test_precomposed_vector.py -v`
  - `python3 schemas/validate.py`
- Non-goals:
  - public CLI/profile publication
  - JPEG/CFF implementationの代替

### MI4-V19 External evidenceとMI4-13 publication readinessを閉じる

- Status: Pending
- Depends on: MI4-V18, MI4-11, MI4-12
- Design inputs: docs/27 §12、§15、§16 step 10、docs/25 MI4-13
- Primary files:
  - `README.md`
  - `contracts/machine-pdf-capabilities.md`
  - `workspace/crates/typaxis-cli/src/machine_tests.rs`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
  - `samples/machine-package/staging/production-book-1/precomposed-vector/`
  - `tools/verify_precomposed_vector.py`
  - `tools/verify_pdf_differential.py`
  - `tools/verify_pdf_structure.py`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `docs/25-machine-input-pdf-improvements-todo.md`
  - `docs/27-vmb-precomposed-math-vector-todo.md`
- Deliverables:
  - MuPDF/Poppler、pinned veraPDF、Matterhorn `/2`で閉じたfeature evidence。
  - JPEG/CFFを含むcomplete production resource-set `/2`とのcombined readiness proof。
  - MI4-13が一changesetで公開するexact Schema/capability/fixture expectations。
- Tasks:
  1. `MI4-11` / `MI4-12` completion後のPNG、SafeVector 1/2、JPEG、TrueType、CFFをfixed component/media orderで合成し、production resource-set `/2` receiptとdescriptor projectionを検証する。
  2. private combined buildをclean targetから実行し、MuPDFの複数DPI raster、Poppler page/text、independent Form/operator/parser、outline/link/language/structure validatorを同じPDF hashへbindする。
  3. pinned veraPDF `ua1`を実行し、new Formula/Figure/number mappingを含むresultをrecordする。warningやtool unavailableをsuccessへ変換しない。
  4. Matterhorn assessment `/2`へmachine-checkable item、human semantic review item、not-applicable reason、tool/version、fixture revision、PDF hashを記録し、未評価itemをpassedにしない。
  5. macOS/Linuxのexplicit managed hostでsame revision/source/fixture/tool inputsからevidenceを作り、binary、PDF、sidecar、resource、font、tool identityをaggregateする。GitHub Actionsを使わない。
  6. expected public capabilityをcomplete merged profile objectとしてfreezeし、blocks/inlines/style/image/vector fields、resource-set component/media order、profile tuple位置、default paragraphをSchemaとfixtureから双方向検査する。
  7. staging directoryへ、V18で拡張したprivate fixture-expectation Schemaに適合する`publication-expectation.json`を追加し、precomposed vector、JPEG/CFF、existing semantic/math/navigation/tagged contentをMI4-13のcomplete `m4-production.json`へ移すexact input ledgerを固定する。public matrix自体はまだ作らない。
  8. `docs/25-machine-input-pdf-improvements-todo.md`のMI4-13 acceptance/verificationへvector capability、SafeVector `/2`、math-vector `/1`、navigation/tagged `/2`、VMB combined/external evidenceをexactに追加する。
  9. README support matrix、capability contract、roadmap/matrix/checklistへprivate implementation complete/public not-yet状態を反映し、public alias/capability/helpはMI4-13まで変更しない。
  10. full locked local quality gate、Schema/Python suite、old contract/profile byte freeze、diff/whitespace/dependency firewallを通し、implementation/evidence commitを記録する。
- Acceptance criteria:
  - complete private production buildが全resource componentとproducer-composed vectorを同時に閉じる。
  - independent renderer/parser/extractor/validatorの全resultが同一PDF hashとfixture revisionへbindされる。
  - veraPDF/Matterhorn evidenceがmissing、stale、warning-only、wrong hashでない。
  - expected capabilityとpreflight accepted setが双方向一致し、public current bytesはまだ変わらない。
  - MI4-13は本milestoneをdependencyとして持ち、残作業がatomic alias/profile/Schema/CLI/docs publicationだけになっている。
- Verification:
  - `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked -- -D warnings`
  - `python3 schemas/validate.py`
  - `python3 -m unittest tools/test_precomposed_vector.py tools/test_pdf_differential.py tools/test_pdf_structure.py -v`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_precomposed_vector_external --locked -- --ignored`
  - `python3 tools/verify_precomposed_vector.py --require-host-evidence target/machine-e2e/precomposed-vector-host-evidence --required-host macos --required-host linux`
  - `/usr/bin/git diff --check`
- Non-goals:
  - MI4-13のpublic alias/current contract switchそのもの
  - M5 long-running fuzz/release governanceの代替

## 6. Requirement traceability

### 6.1 Design section coverage

| Design section | task owner |
| --- | --- |
| §1 document status/publication branch | MI4-V02、MI4-V19、MI4-13 handoff |
| §2 conclusion / §17 rejected alternatives | §1 scope/non-goals、MI4-V02 ADR |
| §3 current gap/version identities | §2.2、MI4-V02、MI4-V05〜V17 |
| §4 Wire contract | MI4-V03、MI4-V04、MI4-V05 |
| §5 baseline equations | §2.4、MI4-V08、MI4-V09、MI4-V11 |
| §6 inline itemization | MI4-V09 |
| §7 block layout/pagination | MI4-V10、MI4-V11 |
| §8 Safe SVG 2 | MI4-V06、MI4-V07、MI4-V13 |
| §9 vector PDF/dedupe | MI4-V07、MI4-V12、MI4-V13、MI4-V16 |
| §10 source/alternative/accessibility | MI4-V04、MI4-V08、MI4-V14〜V16 |
| §11 determinism/manifest | MI4-V07、MI4-V12〜V19 |
| §12 capability descriptor | §2.10、MI4-V05、MI4-V17、MI4-V19、MI4-13 handoff |
| §13 error policy | §2.8、MI4-V03〜V18 negative gates |
| §14 crate ownership | 各milestoneのPrimary files |
| §15 acceptance tests | MI4-V01、MI4-V18、MI4-V19 |
| §16 implementation order | §3 dependency map、MI4-V01〜V19 |
| §18 requirement traceability | §6.2 |

### 6.2 User requirement coverage

| Request | implementation milestones | acceptance/evidence owner |
| --- | --- | --- |
| 1 inline SVG | MI4-V03、V04、V08、V09、V12、V13 | V18 inline corpus、baseline/spacing assertions |
| 2 block SVG | MI4-V03、V05、V08、V10、V11、V12、V13 | V18 alignment/number/page-end cases |
| 3 vector PDF embedding | MI4-V06、V07、V12、V13 | V13 operator/no-raster tests、V19 renderer evidence |
| 4 PDF fragment alternative | 初版non-goalとして§1.2に固定 | V02 ADRがno-import boundaryを採択 |
| 5 math metrics | MI4-V03、V04、V08 | V09/V11 layout equations、V17 manifest facts |
| 6 inline line breaking | MI4-V09 | V18 Japanese/line-end/high-math corpus |
| 7 accessibility | MI4-V04、V14、V15、V16 | V19 Poppler/veraPDF/Matterhorn evidence |
| 8 resource dedupe | MI4-V07、V12、V13 | V18 one Form + N Do / cross-ID alias cases |
| 9 deterministic output | MI4-V07、V12〜V18 | V18 two-build/path-alias、V19 host evidence |
| 10 capability descriptor | MI4-V05、V17 | V19 exact private expectation、MI4-13 public activation |
| 11 error handling | MI4-V03〜V18各negative gate | V18 phase/code/location/side-effect matrix |
| 12 acceptance tests | MI4-V01 corpus、V18 integration | V19 independent/external validation |

## 7. MI4-13 handoff contract

`MI4-V19`完了後もpublic surfaceは変わっていない。master planの`MI4-13`は既存M4 sliceと本機能を同じchange setで次の順に公開する。

1. current 1.3 Schema/contract/capability/manifest/fixtureをfrozen versionとして保存し、byte regressionを通す。
2. complete private 1.4 registryを検証してからcurrent contract/Schema aliasとWire encoder/decoder/`dump-ast` dispatchを切り替える。
3. `production-book-1`を8件profile tupleへ追加し、§2.10のcomplete vector fieldsとresource-set `/2`をpublic capability serializer/preflightへ同時接続する。coarse `image_formats`はexact `jpeg, png, svg`、Safe-SVGのprofile/media差は`vector_profiles` / `vector_media_by_kind`で表し、defaultは`paragraph-1`から変えない。
4. V19の`publication-expectation.json`と既存M4 semantic/JPEG/CFF evidenceから`profiles/production-book-1/combined/`と`matrices/m4-production.json`を作り、全advertised feature/mediaにpositive coverageを持たせる。
5. public `check-package` / `build-package` / `dump-ast` / `capabilities` E2E、Schema、renderer、extractor、tagged validator、two-build、managed-host evidenceをpublic binaryで再実行する。
6. docs/contract matrix/checklist/producer guide/CLI guideをimplemented + E2E completeへ一括更新し、partial profileやprivate staging selectorを残さない。

MI4-13はvector parser/layout/PDF logicを再実装せず、V19でsealedになったreceipt/artifact ownerをpublic dispatchへ接続する。V19 evidenceがstale、wrong revision、wrong PDF hashになった場合は再生成し、publicationだけを進めない。

## 8. Change-set and review gates

### 8.1 Public isolation gate

`MI4-V03`〜`MI4-V19`の各変更後、少なくとも次を確認する。

- current `typaxis_core::CONTRACT`は1.3のまま。
- public `capabilities --format json`は七profileだけで、`production-book-1`、`svg-safe-2`、new vector kind/fieldを含まない。
- public `build-package` / `check-package` / `dump-ast`はprivate contract/profileを選択できない。
- current/frozen 1.0〜1.3 Schema registryとfixture bytesが変わらない。
- public/current pathsへhidden staging flag、environment variable、undocumented aliasを追加しない。

### 8.2 Receipt closure gate

各phaseのpositive testは、少なくとも次のfingerprint edgeを再検証する。

```text
wire/package/session/limits
  -> syntax metrics/source/alternative + computed-language registry /2
  -> admitted SafeVector + profile/style authorization
  -> precomposed vector/math binding
  -> selected inline/block placement (+ MathVectorFlowId for block math)
  -> DrawVector display /2 + selected navigation /2
  -> content-key Form plan /2 + structure/marked-content plan /2
  -> final tagged PDF
  -> SafeVector PDF /2 + navigation PDF /2 + tagged observation /2
  -> SafeVector /2 + math-vector /1 + navigation /2 + tagged /2 manifests
  -> production build manifest
```

各edgeに少なくとも一つwrong-fingerprint、wrong-owner、wrong-versionのnegative testを置く。upstream receiptへdownstream object number/MCIDを先取りせず、manifestだけで欠落closureを補わない。

### 8.3 Final local quality gate

`MI4-V19`をCompletedへする前に次をすべてlocalまたはexplicit managed hostで実行する。

```text
cargo fmt --manifest-path workspace/Cargo.toml --all -- --check
cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked
cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked
cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --all-features --locked -- -D warnings
python3 schemas/validate.py
python3 -m unittest tools/test_precomposed_vector.py tools/test_pdf_differential.py tools/test_pdf_structure.py -v
/usr/bin/git diff --check
```

GitHub Actions、GitHub workflow、workflow dispatchをverificationとして使用しない。

## 9. Task document quality gates

このtask文書自体は次を満たしてから実装入力として扱う。

- design §1〜§18と要求1〜12がmilestone/acceptanceへtraceされている。
- 全milestoneにstatus、dependency、design inputs、primary files、deliverables、tasks、acceptance、verification、non-goalsがある。
- dependency graphがacyclicで、public activationは`MI4-V19 -> MI4-13`だけである。
- decision、corpus、implementation、external evidence、publicationのownerが分離されている。
- current/frozen `/1` isolationとprivate/public boundaryが各cross-cutting milestoneに明記されている。
- verification commandにGitHub Actionsがなく、repository-local package/tool名を使っている。
- 未解決の仮置き語や曖昧な条件分岐がない。

## 10. Review record

- Review pass 1 (2026-09-03): source design、README、docs/21〜25、current M4 code/schema/fixture ownerを再照合した。Display `/2`完成前にForm finalizerを要求するdependency、standalone vector PDF hashをfinal tagged PDF closureへ誤用できるphase境界、computed-language/structure/final PDFの順序、profile authorizationが未完成syntax receiptを参照するdependency、native math manifestとmath-vector manifestのSchema owner混同、MI4-13所有のpublic matrixをV19で先取りするscope重複をfindingとして検出した。
- Review pass 1 fixes: content candidate planningをV07、Display join/Form finalizationをV13、final PDF hash sealをV16へ分離した。V04/V05、V13〜V16のdependencyとruntime phaseを修正し、math-vector専用module/Schema、private publication expectation、private host-evidence Schema/tool boundaryを追加した。
- Review pass 2 (2026-09-03): design内の全versioned/legacy identity、error code、limit name、wire/vector fieldをtask文書と集合比較し、欠落を0件にした。19 milestoneすべてが必須10 sectionをexactly one持つこと、dependency graphが41 edgeでacyclicかつV01〜V19を全て含むこと、要求1〜12とdesign §1〜§18のtrace、new-file list、public isolation、whitespace、仮置き語を再検査した。
- Review pass 3 (2026-09-03): current Schema/Rust exhaustive consumer、ADR-0032〜0036、README、contracts、docs/21〜25と再照合し、coarse `image_formats`へのmedia名混入、equation-number dense NodeId/source order不足、V03 incremental compile gap、same-input determinismの入力順変更、V07/V13でのabsolute PDF object早期採番・二重課金、semantic nonempty不足、related-doc owner不足、currentColor testのpublic到達不能、master/detail plan owner重複、sum/integral corpus不足、実SHA collision fixture不能をfindingとして検出した。
- Review pass 3 fixes: capability語彙を`jpeg|png|svg`とexact vector profile/mediaへ分離し、number child/dense preorder、全exhaustive consumerのfail-closed staging、identical-input schedule permutation、relative object roleからV16 final graphでの一回採番/課金、4 kindのauthored-content規則、README/contracts owner、private different-color test、master stub/link owner、sum/integral、test-only digest seamを設計と全owner milestoneへ反映した。
- Review pass 4 (2026-09-03): 修正後の設計/タスクを再読し、別TextBufferのTextSpan offsetを直接比較するsource-order条件と、V01/V02 gateおよび§16 steps 3〜8がdependency graphと逆転している二件を検出した。順序比較をidentity TextMap対応SourceSpanへ変更し、V01をpre-adoption evidence、V02をV03以降のgateとして§16をV01〜V19 dependency順へ同期した。
- Review pass 5 (2026-09-03): 変更後の要求1〜12、設計§1〜§18、全19 milestone、41 dependency edge、current code/schema/related-doc ownershipを再照合した。relative Markdown link、必須section、全milestone coverage、acyclic graph、stale contradiction search、`python3 schemas/validate.py`、`/usr/bin/git diff --check`を再検証した。
- Final review result: No findings. Implementation remains Pending and must start at MI4-V01.
