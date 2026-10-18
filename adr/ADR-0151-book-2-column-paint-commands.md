# ADR-0151: 実段組paintから元CIDと座標の描画commandsを発行する

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.288](../docs/28-vmb-book-production-compatibility.md#book-2-column-paint-commands-design-14288)
実装記録: [§288](../docs/28-vmb-book-production-progress.md#book-2-column-paint-commands-design-14288)

## 問題

段組resourceは元paint・glyph・CIDと画像payloadを保持するが、font streams／objectsとtext・
image commandsは単段display型に限られている。元段所属を保持したまま、実CID・native式の
個別font size・rule、元physical page coordinatesと別image occurrenceを発行する必要がある。
image allocationは実encoded textの全先行prefixから進めなければならない。

## 決定

font streams／objects、text commands、vector programs、image objects／commandsを共通
Data<D,P>へ拡張する。旧名は具体BodyDisplay／BodyPaint、新Column名は具体ColumnDisplay／
ColumnPaintのaliasにする。明示Pを全parent fieldへ保ち、既存borrowのcovarianceを維持する。
leaf・encoder・sort・sizeof予約・hash順と元単段の有効bytes／counterを共用する。

sealed ResourceDisplayを継承するBookV2CommandDisplayは元text paintとslotのreadonly viewを
返す。viewは実typed paint・page・slot countと、cluster・nativeの元GID／x／y／font size・
ruleの実inkだけを保持する。private constructorを保ち、再shaping・flow公開・座標の再配置や
全paintの追加traversalを行わない。footnote separatorだけfont slotをNoneとする。native
ruleは元slotを保持し、CID／usage／ActualTextを消費しない。

with_budgeted_book_v2_column_paintsは元displayのfont selection・closure・program・CIDから
実ToUnicode／CIDToGIDMap／ActualText、font objectsとtext commandsを作る。実text ownerが
同じdisplayであることを照合し、next_stage_after_countedで全先行records／spool／workを
同じCFF sessionへ移す。その後image selection・raster・vector・image objects／commandsを
作る。image priorは実textの三counter以上、vector prior outputは実text output以上を要求する。
font objectの次からimage objectsを割り当てる。この範囲だけを文書全体のobject認可へ使わない。

各counted constructorはidentity検査と既存予約順を維持し、後続capまたはObjectId検査の
失敗でも受理済みrecords／spool／output／workを返す。stage buildとcallbackの追加操作・
返却error・retryも同じcaller ledgerへ戻す。vector streamのimage-object output creditは
既存の一回消費を維持する。callbackは最後のimage-command builderだけを借用する。

borrowed bundleはtextとimage command roots、元caller開始prefix、八constructor後の実
records／spool／output／workとprivate font bufferのsizeof診断を保持する。追加のallocation
やhashを行わず、未提供のmarked content・structure・navigation・running region・全PDF
認可を作ったとは扱わない。

## 検証範囲

元paint・glyph・CIDから実font payload・object roles・Tf／Tm／Tj・rule・Doとmatrixを照合
する。native分数の実ruleと各glyphのscaled font size、ambiguous glyph・ligature・multi-
substitutionの元textとUTF16 ActualText、実反復headerだけのGID71を含める。manual retained
cutのsource・実段幅・選択方法を保持する。元PNG／JPEG／SVG／vector formulaの別occurrenceと
共有payload、notes・marker・番号・均等化・空／強制ページも維持する。

constructor・stage・callerのexact／1不足と受理済みprefix、失敗・retry・callback、別parent／
limitsを検査する。旧単段PDF bytesと診断counterを回帰する。原CID CFFの正の受入は元Harano
gateを維持し、今回の独立正のfont入力はTrueTypeに限る。SVG parserやPNG deflater全体の
exactな独立再実装、段組PDFの受入は主張しない。

実paint driverの観測はcallerの全先行prefixへ照合する。source／math callbackで手動構築
した経路はlocal builder counterの証拠とし、実ancestor callback値も別に照合する。
その手動操作をsource driverへ加算したとは扱わない。独立counter replayは保持した成功
buildに限る。失敗・retryとoutput creditの再利用はCLI testsへ帰属し、独立attempt traceの
再実装とは扱わない。

## 残る範囲

段組marked content・構造・navigation・running region接続とPDF、Pageラベルの外側の収束、
外側driver retryのCFF session／Type2会計、全commandのphysical byte／capacity／spoolと
同時保持graphを継続する。公開Book /2と4 manifests、元全巻・5,000 distinct画像・原Harano・
管理host・制御性能・著者／人手受入と、元数式の未作成Speech／SemanticRefは未完である。
