# ADR-0155: 実段組のrunning Header／Footerとinspectable PDFを接続する

状態: Accepted（private staging checkpoint。公開・全巻受入は継続）
日付: 2026-10-15
対象: VMB設計のColumn running-region／PDF接続。§14.291 Navigationの正式受入へ含めない。

## 決定

完成したColumnDisplayへ、同じ元source／admitted resources／limits／binding epochに属するPageRegionDisplayをconsumeして結ぶ。Header／Footerは実selected physical pageのnamed／parity masterで選び、完全なpage・Header・Footer順と元owner／frameを検査する。別source、未選択、欠落、余分、重複、順序違い、別page／epoch／frameを受入へ混ぜない。元Column選択は元body／note／balanced geometryを保持し、running textをbody columnへ入れない。

元六streamに加わるPageRegionText paintのpayloadは一つのusizeとする。private O(1) locator vectorが元regionとdrawの組を保持し、旧Column paintの16-byte footprintを維持する。旧paint storageと新paint／region／locator storageの同時保持を予約する。全authored mastersにregionがない場合はbuilder／master scan前に元displayを返し、元fingerprint・bytes・countersを保つ。authoredだが未選択のregionsがある場合は、実master scanとaccepted prefixesを保持し、空joinのpaint／fingerprintは変えない。

running Header／Footerは最初のpaintからPagination artifactであり、MCID／logical binding／body fragment／columnを新設しない。元glyphsをfont selection／CID／text commandsへ渡してからmarked contentを作る。追加glyphsを本文font subsetの外へ落とさない。NavigationはartifactのgeometryへLink rectangles／destinationsを捏造しない。

private ColumnDelivery contextは既に存在するresource policyと共有line-pass ledgerを借用してMath→Displayまで運ぶ。元navigation／binding epochは元prepared sourceに保持された値を借用し、再prepare／fresh ledgerを作らない。public callback signaturesを維持し、counted join後のtyped continue_after_displayはexact source／admissionを検査してbuilderへ実R／W prefixを戻す。数値setterやoutput refund入口を追加しない。

source／line／display／joinとcallbackの全resultでaccepted R／Wとbegun line passesをerror伝播前に保持する。region line allowanceは同じcommandのremaining work／passesを使い、局所workを絶対counterへ二重加算しない。O／spool settlementは後続の既存font／image／marked stagesに属する。

## PDFの決定

元Body PDF writerのbytes／object plan／xref／二回のEncoder計測と予約を共通化し、Body／Columnのfamily型を別aliasに保つ。sealed AssemblyDisplayのprivate source viewは元metadata、Page候補値、実selected page masterとframeの検査だけを公開し、ColumnからPreparedBodyFlowを取得させない。Column frameは元plan／named page／parity master／stable selection／完全body／note／各columnとbalance高さに結び、同じbody寸法を持つ別masterもpointer identityで拒否する。旧Bodyのcolumn guardと従来のframe検査順を保持する。

Column PDF driverは元Navigation callback内で十四番目のcounted constructorを実行する。private output phaseは同じdisplay／text／image commandsと実Navigation pointerに結び、typed Assembly builderが実際に保持したsigned Oを全resultで返す。font／image objectsとmarked bytesの置換creditは測定・予約に成功した一回だけ支出し、後続のWork／allocation／fold失敗、callback内追加buildと外側retryで回復させない。R／S／Wはaccepted maximumを保持し、Oの減少をmaxで消さない。

Navigation pointerだけではcallback内のbuilder置換を識別できない。実safe Rustによる同じNavigationのfresh builderへの置換で、開発候補は既生成PDFのOを7,597 bytes取り消し、fresh buildによる元10,845 bytesのcredit再支出も受理した。旧候補・二つの失敗回帰を保存し、正式freeze前の修正対象とする。accepted constructorごとにprivate fieldを持つopaque attempt identityをchecked atomic counterから発行し、private phaseは最初のbuild前に元builderのidentityを固定する。別attemptのretainをaccepted Oの書換え前に拒否する。identityは比較専用で、numeric setter／Default／外部constructor／Builder Cloneを設けず、PDF bytes・canonical hash・既存予約loopへ混ぜない。独立kernelのfresh builder構築と、同じ元builderでの合法な追加buildを区別する。

attemptだけを固定した次候補にも、元builderの追加build後に置換するとaccepted R87／S19,882／O18,442／W10,392を失う欠陥があった。元failed binary／source／logsと23成功commands・一中断を保存し、その候補を受入しない。最終callbackはprivate fieldとinvariantなfresh loanを持つContinuationの借用だけを受け、buildとreadonly gettersをforwardする。raw builderをdriverが保持し、callback後にその実counterを読み直す。raw取得／置換／loan escape／cross-loan swapの実compile-fail四件とpositive二件、追加build／application Err／late Work Err／借用reference forgetとretryの実全quartetを検査した。新sourceの正式25 commandsは別column-running-regions-final/で全成功した。実結果と証跡の範囲は[実装台帳§292](../docs/28-vmb-book-production-progress.md#book-2-column-running-regions-pdf-design-14292)へ結ぶ。

Book /2の非空semantic containerが元PageBreakだけを含む場合は、その元command spanをcontainer wrapの対象にする。本文glyph／paragraph／fragmentを補わない。空containerとLegacyの従来拒否、通常のphysical source pathは保持する。元失敗は開発証跡に保存し、二／三段とnested／style／keepの三blank-page検査で元二command ownerと全Header／Footerを照合する。

## 検証

二／三段、named／parity／forced blank／notes-only pages、Header／Footer／empty／unselected-only mastersをactual buildersで検査する。missing／duplicate／foreign／wrong order／page／source／limits／epochと各予約／work／pass exact・1不足、失敗後retryとcallback追加buildのprefixを検査する。元no-region Column artifacts／budgetsと元Body PDF bytesを維持する。新region diagnosticsの元JCS／source bounds／glyphs／resource union／artifact slotsは独立oracleで照合する。元Harano positive gateを維持する。

## 継続範囲

正式source freeze／25 command inventory／回帰・独立matrixとpost／readonly audit／closureをcolumn-running-regions-final/へ保持する。独立checkerは元sourceと実glyph／resource commands／構造・navigationからPDF bytesとAssembly R／S／O／Wを再構築する。未再導出のsyntax／shape／line prefixとbuild metadataはissuer factsとして範囲を記す。元Navigation canonical package全件のpublic PDF driver試行を、manual caller／retained contextの再現と同一視しない。

外側Page-label convergence、外側retry CFF／Type2、全command physical bytes／capacity／spoolと同時graphsは別に閉じる。公開Book /2・four manifests、元全巻／画像／fonts／管理host／性能／著者・人手受入と元6,343 Speech／SemanticRef未作成を完了と扱わない。

broader readonly auditで旧standalone Display／Resources／Paints／Marked／Relations／Navigationの六raw-builder callbacksにも追加build後の置換によるaccepted extra prefix喪失のcode経路を認めた。六stageの新実行reproではなくsource-level reachability proofであり、別の未完codeへ追加する。新PDF callbackのparent bundlesはimmutableで、元六builderのmutable escapeはない。従来normal callbacksの検査を任意safe replacementのowner維持へ広げない。


## 補足recovery

正式25 commandsは全成功したが、元runnerはその後のloaded-helper marshal guardでexit1となった。
元runner／logs／reportsを保持し、exact sourceのfresh compileと実loaded codeの再帰descriptor、
実assessの同bytes report、source／environment／binary／artifact guardsによる補足recoveryで欠けた
archive／primaryを発行した。元processのRAM maps回収や元runner成功は主張しない。
recoveryとpost／readonly auditのhash chainは[実装台帳§292](../docs/28-vmb-book-production-progress.md#book-2-column-running-regions-pdf-design-14292)へ結ぶ。
