# ADR-0153: 実段組のsource relationsと脚注の読み順を保持する

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.290](../docs/28-vmb-book-production-compatibility.md#book-2-column-structure-relations-design-14290)
実装記録: [§290](../docs/28-vmb-book-production-progress.md#book-2-column-structure-relations-design-14290)

## 問題

元source hierarchyへ実marked scopesを結んでも、脚注のreading／return edges、元table
Headersとlist numberingがなければlogical structureを完成できない。段組のpaint順から脚注
依存を復元すると、共有branch・後方定義・循環依存と未参照定義の元関係を失う。完成した
marked contentのnet outputを新stageのraw creditへ戻してもならない。

## 決定

source relationsを共通Data<D,P>へ拡張する。旧Body名と新Column名は具体display／paintのaliasへ
保ち、全parent fieldへ明示Pを渡す。元borrowのcovariance、leaf layout・二回のHeaders scan・
六vectorの予約・charged workとhash順を維持する。

sealed SemanticDisplayを継承するRelationDisplayは元tables／lists、definition ownerとreference
registry、実unreferenced countと選択最終source-stateのfirst-referenceだけを返す。private
constructorを持つreadonly viewからprepared physical Body flowへ出る入口を作らない。Bodyと
Columnの選択endpointを具体型のまま区別し、Column stateには元demandの一つのgetterだけを
渡す。source referenceのfieldsとconstructorもprivateへ保つ。

元Reference／Noteのsource／link／label slotsとpainted bindingsへ全edgeを結び、元source
orderでfirst painted return edgeを保持する。読み順の最初のseedは実選択first-reference
forestへ結び、循環seedを拒否する。その後、元定義順に自分のdescendantsへ入らない最後の
painted referenceを選ぶ。共有inline branchに付くNoteの元owner順と、未paint edge／未参照
定義のskeletonを維持する。元hierarchyの全nodesを保持し、depthと全visitを検査する。

table Headersは元tableの先頭Head rowsとlogical column intervalsから導き、colspanの交差へ
対応する元TH／TDの関係を保持する。入れ子tableと未参照脚注定義内の元tableもsource registry
へ保持する。反復physical header artifactへ新logical node／bindingを作らない。元ordered／
unordered listへDecimal／Discを保持する。

with_budgeted_book_v2_column_relationsは実marked driverを共用し、完成したsource structureから
relation builderを作る。counted constructorは先行counter floor、identity、atomic reserveと
work順を維持し、受理済みprefixを失敗にも返す。relation buildとcallbackのrecords／spool／
workをerror伝播前にcallerへ戻す。outputは元marked settlementの実値を保ち、refund／phase
setterを追加しない。borrowed bundleは元marked roots・relations、実marked callback prefixと
十二constructor後counterを保持する。callbackは最後のrelation builderだけを借用する。

## 検証範囲

実Columnのsource order・全node／edge・Noteのreturnとreading choices・selected forest、
logical Headersとlist numberingを元sourceへ照合する。四capのexact／1不足、counted constructor・
build・callbackの失敗prefixとretry、二回buildの予約と不変outputを検査する。旧Body helpersと
実PDF bytesを回帰し、Column／Bodyの型変換とprivate view構築を拒否する。

独立checkerは元JCSと既存の独立marked検査へ全relationsを結び、元registry・logical Headers・
list facts、relation hierarchy／edgesと元hash folds、保存した成功buildの予約・workを導く。
選択first-reference demandは実最終source-stateのissuer factsへ結び、元source demandの探索
kernel全体を独立再実装したとは扱わない。relation productはserialized canonical bytesを保持
しないため、元公開recordsからhash preimagesを再構成した証拠とする。失敗・retryの検査はCLI
assertionsへ帰属する。原CID CFFの正の受入は元Harano gateを維持する。

継承するraster paintsは実encoded bytesとspoolをexactに照合する一方、deflaterの追加records／
workは元checkerのbounded accepted prefixを保つ。relations自身の予約・hash workのexact
再生を、上流全commandのrecords／workが独立exact再生されたという主張へ広げない。

凍結1,643 inputsの正式20 commandsは全成功した。CLIは379成功／106 ignored、独立relationsは
全60 actual probesと57,249 tamper拒否で成功し、元637 PDF bytesを維持した。元markedの
74,428拒否は同じ49 inputs／programs／environmentに結ぶ§289のretained full proofであり、
今回のfresh normal marked検査のnegative数は0である。詳細と残るgatesは実装台帳§290へ保持する。

## 残る範囲

段組navigation・running region接続・PDFと独立受入、Pageラベルの外側の収束、外側retryの
CFF session／Type2会計、全commandのphysical byte／capacity／spoolと同時保持graphを継続する。
公開Book /2と4 manifests、元全巻1 package／1 PDF・5,000 distinct画像・原Harano・管理host・
制御性能・著者／人手受入、番号付きtable headerの上流一時spoolと元数式の未作成
Speech／SemanticRefは未完である。
