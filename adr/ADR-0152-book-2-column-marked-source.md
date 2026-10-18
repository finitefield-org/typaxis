# ADR-0152: 実段組paintのmarked contentを元source hierarchyへ結ぶ

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.289](../docs/28-vmb-book-production-compatibility.md#book-2-column-marked-source-design-14289)
実装記録: [§289](../docs/28-vmb-book-production-progress.md#book-2-column-marked-source-design-14289)

## 問題

段組の実CID／画像commandsを描画できても、元semantic owner、pageごとのMCIDと元source
hierarchyの対応がなければ構造へ接続できない。反復headerはartifactとして扱い、Formulaの
親ActualTextとCID-local replacementを区別する必要がある。marked contentのoutputは既存
raw contributionsを置換するため、単純な最大値だけのcaller会計では正しいnet値を失う。

## 決定

marked scopes／contentとsource structureを共通Data<D,P>へ拡張する。旧Body名と新Column名は
具体display／paintのaliasへ保ち、全parent fieldへ明示Pを渡す。元leaf layout・encoder・予約・
hash順と単段の有効bytes／counterを共用し、段組chainを単段PDF型へ変換しない。

sealed CommandDisplayを継承するSemanticDisplayは実Body／Columnのreadonly viewを返す。
元paint・owner・physical page・global fragment・role・artifactと元Alt／ActualTextを保持し、
textの元全文とimageの実baselineを必要な箇所だけへ渡す。private constructorを維持し、
座標を再配置しない。syntax viewは保持した元sourceのvisitor・language factsだけを公開し、
prepared physical body flowを公開しない。node文字列のborrowはtemporary viewへ短縮しない。

scopeは同じowner・page・fragment・role・artifactの連続Textだけをまとめる。Formula、
Figure、Labelと別streamの式番号は別scopeとする。semantic MCIDはpageごとに0からdenseに
割り当てる。反復headerと脚注separatorはMCID・Alt／ActualText・logical bindingを持たない。
Formulaは元Speechと親ActualTextを保持し、内側SpanへUTF16 replacementを発行する。
native glyph／ruleとvector Formulaの実paintを保持し、必要なimage replacementだけへ元
viewport／baselineの非描画Type3 anchorを発行する。Speech／Altはscope／source metadataへ
保持し、最終structure objectでの/Alt property発行はこのstageへ計上しない。

with_budgeted_book_v2_column_markedは実paint pipelineを共用し、scopes・content・source
structureを順に作る。same-display、実textの全先行prefixとfont／image object rangeの
非重複を検査する。borrowed bundleは実paint roots・完成したcontent／structure、settlement前の
実encoded observationと11 constructor後のcounterを保持する。callbackは最後のsource
structure builderだけを借用する。

records／spool／workは受理済みprefixをerror伝播前にcallerへ戻す。outputはprivate typed
phaseを使い、Scope build後の実outputを結果にかかわらず保持する。Content build後も実
builderのoutputを結果にかかわらず保持する。最初の受理済みcontent予約でraw text／image／
scope bytesと未使用CID ActualTextのcreditを一回だけ消費し、net outputが減る場合も保持する。
後続work／allocation失敗はその予約を取り消さず、予約前の失敗はraw prefixを保持する。
公開paint callbackへ数値refund setterを追加しない。

source structureは完成したcontent budgetを直接継承し、constructorのatomic reserveと元
work順を維持する。source visitorから元key／slot・parent・role・language・span・Alt・table
cellを作り、元順序のhierarchyとscope bindingsを保持する。generated labelsと式番号の元
children、未参照脚注定義の元skeletonも維持する。artifactへlogical bindingを作らず、
source structureでraw output creditを復活させない。

## 検証範囲

実Column componentのowner・fragment・MCID・artifact、元FormulaとCID-local ActualText、
別Figure occurrence、page content／Type3 anchorsと元AST hierarchy／bindingsを検査する。
四capのexact／1不足、constructor・build・callbackの失敗prefix、retryと一回creditを検査する。
旧単段PDF bytes／診断と既存独立checkerを回帰する。原CID CFFの正の受入は元Harano gateを
維持する。

独立checkerは実元package JCSとUTF8 buffersへsource hierarchyを結び、元paint／commandsから
scope・content・anchorを導く。保存した成功buildの予約・work・hashを照合する。元shaping／
数式／page kernelとSafeSVG parser全体、失敗attempt traceの独立再実装は主張しない。
syntax flow fingerprintは元issuerのfactへ結び、syntax／style／math preparation全体のfingerprint
計算を独立再実装したとは扱わない。元source SHAはdecoded identity mappingsで全bytesを再構成
できた範囲だけへ照合し、hostの元fileを再readしたとは扱わない。
source nodes／bindingsを作ったことだけで段組PDF structure treeの受入とは扱わない。

先行PNG rasterの出力bytes／spoolはexactに照合し、deflater Writeによる追加allocation
records／workは元checkerの上下限検査を維持する。新三stageのexactな予約／workはその
受理済みpaint prefixから照合し、codec Write境界の独立replayへ読み替えない。

## 残る範囲

選択source relations・table Headers・脚注relocation、段組navigation・running region接続と
PDF、Pageラベルの外側の収束、外側retryのCFF session／Type2会計、全commandのphysical
byte／capacity／spoolと同時保持graphを継続する。公開Book /2と4 manifests、元全巻1 package／
1 PDF・5,000 distinct画像・原Harano・管理host・制御性能・著者／人手受入と、元数式の未作成
Speech／SemanticRefは未完である。
