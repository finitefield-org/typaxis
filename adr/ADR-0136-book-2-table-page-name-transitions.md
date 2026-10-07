# ADR-0136: Book /2 表内の名前付きページ遷移

- 状態: 実装
- 日付: 2026-10-07
- 関連: [設計 §14.273](../docs/28-vmb-book-production-compatibility.md#14273-表の実セル位置から名前付きページを切り替える実装追補)

## 問題

元本文の名前付きページ選択と、並列表の実内容が同じ名前を使う経路は存在する。
しかし `BookV2PreparedBodyFlow` は root table の全 leaf を一つの名前へ制限し、
caption・後続行・各セル内の領域・入れ子表で名前が変わる入力も
`PendingNamedPage` として配置前に拒否していた。

表の共通 offset は、独立したセルの配置が始まると累積 fragment 高になる。
この値から元セルの次の内容やページ名を求めることはできない。
元のセル位置を無視して名前だけを変更すると、異なる master の内容を同じ
物理ページに置いたり、先に終わったセルの内容を再度消費するおそれがある。

## 決定

名前が変わる表には、既存の入れ子表の独立セルカーソルを使う。平坦な表も
同じ経路を使用できる。caption の直列カーソル、各セルの次の元内容、子表の
継続カーソル、元 header と反復 header、rowspan の未配置 band を保持する。
同じ名前だけを使う表には従来の kernel と keep policy を維持する。

ページの開始時に、次の元内容から名前を求める。caption が残る場合はその
直列位置を使用する。行内では現在活動中の各セルの次の内容を調べる。
継続前の名前の内容が残っていれば、その名前を完了させる。先に次の名前へ
到達したセルは待ち、内容を消費しない。元の名前の内容がなくなれば、残る
セルが共通して要求する名前を次の物理ページへ渡す。ここで別々の名前を
同時に要求する場合は元 owner 付きの衝突診断を返す。unnamed は実際の
source 指定として扱い、他の名前へ読み替えない。

内容の選択は活動中の名前で止まる。元 keep がこの境界を越える場合は
`KeepAcrossForcedBreak` を返す。caption と本文、元 header と本文の境界も
名前変更に対応する。後続ページの反復 header は既存の Artifact として
扱い、元 header の再消費や新しい名前要求に数えない。元の明示改ページは
名前変更とは別に一度消費し、その空ページも保持する。

空の子表は leaf を持たなくても元の scope とカーソルを保持する。
rowspan の元 band 高を縮めず、本文の paint が完了しても未配置 band が
残れば、その継続ページを作る。表の終了後は後続本文の元 scope へ戻る。
名前選択を既存の物理 master・幅 feedback・header variant・ページ安定性・
source closure・PDF 経路へ接続する。

## 課金と identity

名前照会、活動行・セルの走査、子表照会、実 subtree の遷移判定には search の
累積 work を使う。親が名前を変えても、均一な子表自身の keep policy を
変更しない。各 trial と照会は caller と子 search の ledger を交換し、
失敗時も ledger を戻してからエラーを伝える。

継続カーソルは元 measurements と source table に結び付く。遷移する表の
保持位置 fingerprint には、セル位置・子の fingerprint・残る band と
実際に選択した名前を含める。この追加 encoding の bytes を既存 spool guard
へ含める。均一な表の encoding は維持する。完全な command allocation・
spool・work の受入を本変更だけで主張しない。

## 検証と範囲

合成フォントと原ノ味無変更フォントで、平坦・非対称セル・入れ子・均一な
子表の連続・行境界・元／反復 header・rowspan・caption・空の子表・元改ページ・
外側 scope への復帰・unnamed・可変幅を検証する。元 item の一回消費、実 master
との一致、driver work と部品 search の work／record の exact・1不足を確認する。
keep と同時に異なる名前を要求するセルの診断も検証する。

独立 Python 検査は元 fixture・宣言 rule・固定 font metrics からページと文字の
配置を求め、実 PDF の text・MediaBox・TrimBox・反復 header の glyph と Artifact
を照合する。配置結果を期待座標として流用しない。全回帰と byte 比較の実結果は
[進捗 §273](../docs/28-vmb-book-production-progress.md#book-2-table-page-name-transitions-design-14273)に記録する。

名前付き脚注定義、段組、公開 Book /2 の一式 gate、元全巻の単一 PDF、管理ホスト・
性能・著者／人手受入は本変更の完了条件とは別の未完要件として継続する。
