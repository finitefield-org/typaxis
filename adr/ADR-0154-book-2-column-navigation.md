# ADR-0154: 実段組のnavigation geometryを元sourceへ結ぶ

状態: Accepted（private Book /2 staging architecture）
日付: 2026-10-15
対象: [VMB設計 §14.291](../docs/28-vmb-book-production-compatibility.md#book-2-column-navigation-design-14291)
実装記録: [§291](../docs/28-vmb-book-production-progress.md#book-2-column-navigation-design-14291)

## 問題

段組のselected displayとsource relationsを得ても、元Anchor／Link／outlineへ実座標を結ばなければnavigationを完成できない。脚注のreading treeをLink継承へ使うと、relocated Noteのinkを参照元へ含める。別段の同じsource ownerのrectanglesをまとめてもならない。

## 決定

NavigationGeometry／BuilderをData<D,P>へ拡張し、旧Body名とColumn名を具体display／paintのaliasへ保つ。全parent fieldへ明示Pを渡し、元borrowのcovarianceとleaf layoutを維持する。元七vectorの予約／capacity／sizeof、全charged loops／workとhash順を変えない。

sealed RelationDisplayを継ぐNavigationDisplayは、元paragraph sites／Anchor／outlineのreadonly private source viewと、actual selected paintのO(1) logical boundsを返す。positioned Anchorとnonpainting lineはprivate fieldsのCopy viewへ保ち、physical paint familyを型に残す。prepared physical Body flowを返すAPIを加えない。Bodyの元flow()とoriginal_graph()は同じflowを借用するため、元source slicesを差し替えない。

Textは元logical bounds、native mathは元receipt bounds、vector mathとimagesは元viewport、markersはplacement bounds、式番号は元number geometry boundsを使う。glyph ink unionへ読み替えない。separator／unexpected running regionをsemantic geometryへ混ぜない。反復Header artifactsは元bindingsがないためLink rectangleへ入らない。

Anchor／Note／Reference destinationsは元registry orderへ保持し、最初の(page,global fragment)の実座標へ結ぶ。実空lineはline boxをdestinationへ使い、Link rectanglesやMCIDを捏造しない。inline Anchorは元baseline／xを使い、未配置AnchorはNoneを保つ。painted Linkまたはoutlineが必要とする未配置destinationだけをUnplacedDestinationとして拒否する。

Link継承は元SourceStructure parent treeを使い、Noteのrelocated reading treeを使わない。重複するactive Linkは元owner付きConflictingLinksで拒否する。同じ(page,global fragment)だけをcoalesceする。Column displayは一つのglobal fragmentを一つの段へ結ぶため、別段をunionしない。空／強制blank pagesにも実page rangeを保持する。Footnoteのforward／return edges、unpainted Notesとoutline sibling／child／descendant factsを元sourceへ保つ。

counted constructorは完成したRelationsのR／S／settled O／Wとcaller floorsをidentity前に返し、copied budgetへatomic reserve(1,0,0)を適用した後にwork ceilingを検査する。失敗にも受理済みprefixを返す。新driverは実relation callbackを共用し、十三constructor observationsとactual relations prefixを保持する。build／callbackの全resultでR／S／Wをcallerへ戻す。outputはmarked ancestorの実settlementを保ち、raw credit／refund／phase setterを追加しない。

## 検証

元Body navigation helpersと実PDF bytes、family変換とprivate構築拒否を回帰する。二／三段とbalance、Link across columns／pages、元六paint streams、empty／inline／unplaced anchors、URI、Text／Number refs、outline、Note returnとreading、各capのexact／1不足、constructor／build／callback failure prefixesとretryを実buildersで検査する。

独立checkerは通常各probeの元Relationsを全再生し、元JCSからparagraph sites／Anchor／Number／outlineを導く。元Text labelsはexplicit outline labelまたはprepared Headingだけへ、Number labelsは元bindingのUTF-8 sliceへ結ぶ。実physical geometryと元source ancestryからdestinations／Link rectangles／outlineと七vector予約・hash workを導く。保存した成功snapshotとactual source semantic refusalの範囲を分け、一般のresource／retry／callback失敗はCLI assertionsへ帰属する。

上流Display diagnosticのBreak recordは元ownerを持たない。placed inline indexをsource-site indexとみなさず、同じselected width-specific graph／paragraphのunique linesが元authored controlsの全censusを保持する場合だけ元順序へ結ぶ。反復line metadataの一致を要求し、partial censusはfail closedとする。これは条件付きdiagnostic joinであり、選択line位置／fingerprintはissuer-linked factsへ保つ。raw source／geometry controlsはwhole-probe mutation数から分ける。

凍結1,648 inputsの正式21 commandsは全成功した。CLIは398成功／107 ignored、新navigationは89 snapshots／535,808 whole-probe tamper拒否と別source controls 5／geometry controls 3で成功した。85 resultsと4 semantic refusalsを区別し、元637 PDF bytesを維持した。旧markedの74,428拒否とrelationsの57,249拒否は、同一inputs／programs／environmentへ結ぶ元§289／§290のretained proofであり、今回のfresh normal検査の新negative数は両方0である。counter／source／scopeの限界と各manifestは実装台帳へ保持する。

## 残る範囲

Page-formatのauthored／candidate labelはfinal placementの証明ではなく、外側のcounted convergenceを別に閉じる。Column running-region join／PDF objects・annotations・serialization、public Book /2とfour manifests、外側retry CFF／Type2、全command physical bytes／capacity／spoolと同時graphs、元全巻／5,000画像／原fonts／管理host／性能／著者・人手受入をこのstageの受入へ含めない。上流raster deflater追加records／workはbounded accepted prefixを継承する。
