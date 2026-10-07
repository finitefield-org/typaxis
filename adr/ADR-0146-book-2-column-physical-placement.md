# ADR-0146: 実段の物理配置と配置一致を累積予算へ接続する

状態: Accepted（private Book /2 staging）
日付: 2026-10-07
対象: [VMB設計 §14.283](../docs/28-vmb-book-production-compatibility.md#book-2-column-physical-placement-design-14283)
実装記録: [§283](../docs/28-vmb-book-production-progress.md#book-2-column-physical-placement-design-14283)

## 問題

§14.282で収束した元sourceと実ページ選択だけでは、段ごとの物理座標、反復headerの
実graph、脚注・list marker・式番号の配置一致を証明できない。本文全幅から得た最初の
行をそのまま段へ移すと幅が合わない。単段の配置型へ変換すると、段組専用の測定・
予算と後続のsource closure／PDF認可を区別できない。

## 決定

既存の本文・表・脚注のleafとmarker配置を一つのprivate kernelへ抽出する。単段は
従来の一つの本文領域を渡し、段組は選択された各段の実boundsとparts、一つの元脚注
領域を渡す。元table cursor・cell・rowspan・caption・反復headerを同じ処理で配置し、
段ごとの連続fragment rangeを記録する。空段も元boundsと空rangeを保持する。

段組はfragmentが属する段／脚注領域と、元sourceまたは反復headerの実測定起点から
横方向の差分を求める。fragment・viewport・list／脚注markerを同じ差分で移し、
fragment boundsの領域内包含を確認する。式番号は全ページのfragment indexを保つ。
headerは元graphを借用する段組専用viewで公開し、単段の測定・flowの認可を渡さない。

BookV2ColumnPlacedSequenceとBookV2ColumnStablePagesを段組専用の型にする。共通の
安定比較kernelで二回以上の実選択と配置を行い、元選択・段bounds・fragment・cell・
反復caption・実header association・marker・式番号・separatorの一致を課金して検査する。
別searchのsequenceを拒否し、開始済みpassと途中失敗の受理済みprefixを保持する。
選択されたmasterがlast_page balanceを要求する場合はcolumn_balanceの未実装診断を返す。
均等化指定を黙ってsequentialの安定配置として受理しない。

with_budgeted_book_v2_column_pagesの元source選択APIと予算の挙動を維持する。共通の
source幅収束driverにwith_budgeted_book_v2_column_placementを追加し、幅が一致した後に
物理配置の安定検査と幅feedbackの再照合を行う。callbackのpages／search／feedbackは
同じgraphのlifetimeを明示的に共有する。追加配置・callback消費も同じcallerへ戻す。

## 検証

元の選択APIと新しい物理APIを同じ回帰で検査する。新しい6 testsは非零の段／脚注起点、
空anchor、入れ子・rowspan・caption・反復headerの本文／脚注10ケース、vector／native式・
PNG／JPEG／SVGの表内外20ケース、原Haranoによるordered／unordered listと脚注を確認する。
元glyphへの参照、実描画幅・baseline、markerとfragmentの所属を照合する。行の使用可能幅と
required_inline_sizeから得る描画幅を区別する。record／workのexact／1不足、callbackでの
追加消費と失敗後の再試行、別owner、短いpass上限、均等化要求の拒否を検査する。
compile-failで単段の安定配置型への変換を拒否する。既存PDFを独立検査し、直前の実装と
byte比較する。source・font・binary・commandsと結果を実装台帳へ対応付ける。

## 残る範囲

実配置の一致はsourceを一回だけ消費したclosure、元数式terminals、balanceやPDFの
receiptを兼ねない。段組PDFと独立受入、Pageラベルから最終PDFまでの外側の収束、
全commandのphysical byte／capacity／spool・同時保持graphの監査、公開Book /2と全manifest、
元全巻・5,000 distinct画像・原Harano全巻・管理host・制御性能・著者／人手受入を継続する。
元本文内数式の未作成Speech／SemanticRefを補ったとは扱わない。
