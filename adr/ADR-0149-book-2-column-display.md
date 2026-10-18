# ADR-0149: 段組の実source graphから本文と脚注の全paintを作る

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.286](../docs/28-vmb-book-production-compatibility.md#book-2-column-display-design-14286)
実装記録: [§286](../docs/28-vmb-book-production-progress.md#book-2-column-display-design-14286)

## 問題

§14.285までの段組は実幅・均等化・source一回消費と数式terminalを保持するが、本文・表と
脚注の全paintをdisplayへ結んでいない。数式だけを描画したり、単段のsource型へ変換したり
すると、元文字・list／脚注marker・独立式番号・通常画像・anchorと実段配置の対応を失う。
反復headerは実幅によって元本文とは異なる行graphを持ち、block画像と式番号はその実block
graphも必要とする。

## 決定

BookV2ColumnDisplayBuilderは同じBookV2ColumnMathTerminalsとresource admissionを保持する。
BookV2ColumnDisplayは本文・表と一つの共有脚注領域のtext・math・独立式番号・marker・
image、非描画anchorと空行を保持する。math drawはBookV2ColumnMathTerminalそのものを
借用する。各componentのsourceは段組terminal型に固定し、単段display／componentへの
変換を提供しない。

単段と段組はprivate Projectorで実page・fragmentとsource graphを借用し、既存のtext／native
math／number／markerのgeometry kernelとvector content key／matrixを共用する。
componentのcollector・固定byte表現・元描画順とwork位置を共用し、単段の有効bytesと
課金を維持する。source固有のresultはprivate constructorを持つ型へ包む。

BookV2SourceGraphは実linesとblocks、limits検査だけを公開し、prepared flowを公開しない。
空documentでも元graphを取得できる。段組terminalの反復header cacheは、従来の行pointer
と同じ予約・容量の実flow pointerを内部に保持し、publicにはreadonly graphを渡す。
constructorは元graphと実header graphを同じadmission／limits／epoch／vector bindingへ
照合する。描画時に再shapingしたり、別headerの行番号を元行として解釈したりしない。

separator・marker・text・math・image・numberの六つのstreamを六つのstack cursorで
合流する。順序はglobal fragment、separator／marker／inline／number、inline indexとし、
同じinline paint keyの重複を拒否する。anchorと非描画行はpaint streamへ挿入しない。
全実fragmentの段所属を元のdisjoint rangeから検査し、共有脚注だけをNoneとして保持する。
source terminal fingerprintは全N段・空段・forced blank・均等化receiptを保持し、displayは
各componentのfingerprintと実順序・段所属をさらに束縛する。

with_budgeted_book_v2_column_displayは既存の元幅・header・均等化・物理安定・source／math
driver上でdisplayを作る。constructor・全component・合流・段所属arrayのrecords／workを
確保前に予約し、途中失敗・再試行とcallbackの追加buildでも受理済みprefixを保持する。
新driverはcallback後のbuilder台帳を元callerへ戻す。既存source／math driverのencodingと
課金を維持し、このdisplayのarrayを全commandのphysical byte／capacity／spool受入とはしない。

## 検証

元cluster・font・GID・source spanと実glyph座標、vector／native式・別number stream、
PNG／JPEG／SVG、list／脚注marker・separator、anchor・空行を元graphへ照合する。
本文／表／脚注、実幅の入れ子反復header、named／parity、均等化と指定なし、空／forced
ページを検査する。records／workのexact／1不足、constructor・build・callbackの失敗prefixと
再試行、別admission／limitsを検査する。型変換拒否をcompile-failで確認し、既存単段の
displayとPDFを回帰する。独立検査は実source／terminalとdisplay probeの対応を検査する。

反復部分だけのcontextual GIDは、実100pt段幅で元source cutを保持したtyped column seed・
実header graph・安定選択・source closure・terminalから検査する。元31pt vector atomを
持つ同じ入力で、18pt未満の自然幅がそのcutを発見した証拠とはしない。probeはこのmanual
retained_source_cutと、既存source callback・新display callbackの会計範囲を区別する。
name-keyed CFFのpipeline前拒否を、CID CFFの正のdisplay受入へ計上しない。

## 残る範囲

このdisplayは段組のresource選択・subset／CID・marked content・構造・navigation・PDFの
認可ではない。running regionの実選択ページへの接続、Pageラベルから最終PDFまでの外側の
収束と全commandのphysical byte／capacity／spool・同時保持graphの会計を継続する。
公開Book /2と4 manifests、元全巻・5,000 distinct画像・原Harano全巻・管理host・制御性能・
著者／人手受入と、元本文内数式の未作成Speech／SemanticRefも未完である。
