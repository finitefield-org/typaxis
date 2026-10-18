# ADR-0150: 実段組paintのresourceを同じdisplay型へ閉じる

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.287](../docs/28-vmb-book-production-compatibility.md#book-2-column-resources-design-14287)
実装記録: [§287](../docs/28-vmb-book-production-progress.md#book-2-column-resources-design-14287)

## 問題

§14.286の段組displayは元glyphとpaintを保持するが、font／image useと後続のselection・
closure・subset・program・CIDは単段のdisplay型に限られている。単段型へ変換すると、
実反復headerだけのglyph、共有脚注、式番号と元paintの所属を失い、既存PDFの認可を混ぜる。
同じpayloadを使う別の元occurrenceは、resource共有後も独立したuseを保持する必要がある。

## 決定

sealed BookV2ResourceDisplayは実BodyDisplayとColumnDisplayだけが実装する。元ordered paint、
admission、fingerprint、counterと元font-instance tableの事実をreadonlyで渡す。font／image
useはprivate constructorを持ち、実familyのpaint indexを保持する。共通helperは実fragment
graphと元native instanceを照合する。単段running regionの既存branchを維持する。

selection・closure・program・CIDとimage・rasterのparent chainは同じdisplay型Dを保持する。
glyph closure・TT／CFF subset・font programとimage payloadの具体kernelは共用する。
旧public名を単段aliasとして維持し、段組aliasを別に提供する。private constructorでのみ
parentを作り、段組chainを既存単段PDF constructorへ渡す変換を提供しない。

containerはpaint型Pを明示して保持し、単段／段組aliasは具体paint型を指定する。
D::Paintをfieldへ直接埋めてDをinvariantにする変更を避け、旧単段のborrow covarianceを
維持する。元loop、sizeof予約、hash domainと順序、sort・CID・ActualText判断を維持する。
元font-instance tableを再集約して置き換えない。glyphは実paintだけから選択する。

counted constructorは受理済みrecords／spool／workを失敗時にも返す。resource driverは
各stageの失敗前にcounterをcallerへ戻す。next_stage_countedは同じbuilderを消費し、
image constructorの1 recordを加え、同じCFF session・limits・displayとcacheを移す。
callbackは最後のbuilderだけを借用し、追加操作も全先行prefixの後から課金する。

resource bundleは二つのtyped rootを借用するreadonly viewである。実caller開始prefixと
二つのconstructor後counterを保持する。このresource-only経路のimage priorはCIDの後で
あり、encoded font／textをまだ持たない。段組PDF経路では実encoded textのbranchをimage
allocationより前へ置き、別のPDF認可と収束を構築する。

## 検証

元paintから独立にfont／image use、実GID集合、元TTのcomposite依存とadvance、subset・
CID／Unicode／ActualTextを照合する。実反復部分だけのGID71は同じ元font・sourceと実
100ptのretained cutを維持する。本文・表・脚注・marker・番号・native式、実header、均等化・
named／parity／残余幅と空／強制ページを検査する。同じbytesの別declared image IDと元
occurrenceを保持し、PNG／JPEG／SVG／vector formulaのpayload共有と別useを検査する。

constructorと各stageのexact／1不足、失敗とretry・callback、別parent／limitsを検査する。
型変換と偽のsealed trait実装をcompile-failへ含め、旧単段の診断counterとPDF bytesを回帰
する。独立検査は元byte・graph・glyphから実resourceとstage証拠を照合する。今回の正のfont
入力はTrueTypeに限り、原CID CFFの正の受入は原Harano gateを維持する。SVGの寸法・decoded
見積りは元admissionのreadonly metadataへ照合し、SafeSVG parserを独立再実装したとは
扱わない。PNG deflaterの出力write境界はprobeに保持しないため、その予約・work suffixは
上下限を検査する。codec全体のexactなwork再実装や段組PDFの受入は主張しない。

## 残る範囲

段組font stream／objects・encoded textとvector／image commands、marked content・構造・
navigation・running region接続・PDFと独立受入、Pageラベルの外側の収束を継続する。
外側driver retryのCFF session／Type2会計、全commandのphysical byte／capacity／spoolと
同時保持graphも継続する。公開Book /2と4 manifests、元全巻・5,000 distinct画像・原Harano・
管理host・制御性能・著者／人手受入と、元数式の未作成Speech／SemanticRefは未完である。
