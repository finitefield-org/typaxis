# ADR-0137: Book /2 脚注定義内の名前付きページ

- 状態: 実装
- 日付: 2026-10-07
- 関連: [設計 §14.274](../docs/28-vmb-book-production-compatibility.md#14274-脚注定義の元要求から名前付きページを選ぶ実装追補)

## 問題

名前付き本文と名前を指定しない脚注の継続は存在する。しかし page plan は最初の
脚注定義で source の走査を止め、collector は名前を指定する定義内容をすべて
`PendingNamedPage` として拒否していた。本文と同じ名前の脚注や、開始済み脚注が
別の master へ継続する場合も、実 source cursor に基づいて配置できなかった。

## 決定

page plan は元の脚注定義まで走査し、元 owner の要求を保持する。本文・脚注の
全 item と空の表を同じ元インデックスに結び付ける。定義内の要求は本文の名前を
置き換えない。明示された名前は、その元内容を消費する物理ページの名前と一致
しなければならない。名前を指定しない定義内容には独立したページ要求を追加せず、
選択された本文・継続ページに従わせる。本文の unnamed は従来の source 指定を保つ。

定義の実直列位置、caption・並列セル・入れ子表の実継続位置から次の要求を求める。
一つの fragment は名前の境界で止まり、後続の元 item を消費しない。本文が残る間は
本文の要求を優先し、開始済みの別名の脚注は待てる。本文終了後は pending queue の
次の元内容が求める名前で継続する。本文と同じページに収まらない既存 carry は、
その名前の脚注専用ページを試せる。本文 cursor は採択するまで変更しない。

新しい参照に対応する定義は、参照と同じ物理ページで最初の実 fragment を必要とする。
開始済み carry は、別の定義が進むページで容量に収まらなければ待てる。新規定義の
最小 fragment と、選択済み内容から発生する依存関係の閉包は免除しない。全 carry を
進めない空の候補を、この待機処理で成功させない。

元 keep が名前境界を越える場合は `KeepAcrossForcedBreak`、新規定義と本文の名前が
一致しない場合は元 owner 付き `PendingNamedPage` を返す。元の改ページ・番号・
参照・空の子表・rowspan の band は一度ずつ保持する。元 header は一度消費し、
反復 header の Artifact は後続 master で描画する。文字を持たない rowspan の継続
ページは残し、脚注の描画 fragment がないページでは区切り線を出さない。

## 課金と検査

元 item の名前 metadata は定義を含む全件について、予約・確保より先に既存の
record ledger へ計上する。名前照会・境界走査・表の実カーソル照会は同じ累積 search
work を使う。caller と子 search の ledger を交換し、失敗時にも戻す。候補の再試行は
消費済み record・work を返金しない。公開部品 API の raw source 要求と、実表 cursor の
照会を分け、前者を物理ページの割当 receipt と扱わない。

合成フォントと原ノ味の無変更フォントで、同名・直列遷移・名前への復帰・semantic
scope・改ページ・平坦表・入れ子表・元／反復 header・rowspan・caption・空の子表・
幅変更・複数定義の各13種類を実 driver で検査する。元 item と番号の一回消費、
実 master との一致、driver work と source constructor record の exact／1不足、
元 owner 付き拒否と失敗後の再試行計上を確認する。

独立検査は元 fixture、宣言 rule と固定 font metrics からページ列・元文字数・番号を
求め、実 PDF の文字、物理 frame、MediaBox・TrimBox、反復 header の文字を照合する。
記録された配置座標を期待値に使わない。共通 PDF 検査も、反復 Artifact と区別して
元定義の明示された名前を検査する。全回帰と byte 比較の結果は
[進捗 §274](../docs/28-vmb-book-production-progress.md#book-2-named-footnote-definitions-design-14274)に記録する。

完全な command byte／spool／work・同時保持 graph、段組、公開 Book /2 の一式 gate、
元全巻の単一 PDF、管理ホスト・制御性能・著者／人手受入は継続する。
未作成の Speech／SemanticRef を生成済み・著者承認済みとは扱わない。
