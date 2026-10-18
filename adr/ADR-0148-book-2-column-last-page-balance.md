# ADR-0148: 元source境界から段組の最終ページを均等化する

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.285](../docs/28-vmb-book-production-compatibility.md#book-2-column-last-page-balance-design-14285)
実装記録: [§285](../docs/28-vmb-book-production-progress.md#book-2-column-last-page-balance-design-14285)

## 問題

元段幅の収束と安定配置は最終ページの均等化を行わない。本文・表・脚注を再配置するとき、
行数や平均高だけではkeep、caption、反復header、独立セルと入れ子の実source境界を保てない。
短い試行容量での不適合を元表のoversizeへ置き換えたり、失敗した候補の予算を返金したり
すると、元入力と限界値に束縛された選択の証明にならない。

## 決定

全高の元ページ列から正の本文使用高または参照脚注断片を持つ最後の物理ページを決め、そのmasterが
last_pageを要求し、元本文使用高の正の合計Eがある場合だけ均等化する。段数Nに対する
ceil(E/N)を最初の高さとし、原full height Hの範囲で試行する。本文と脚注の元終端、名前、
強制空ページ状態を全高選択と一致させる。先行ページを保持し、末尾の空ページは元source
状態から再選択する。脚注だけの最後のページから前の本文ページへ適用対象を移さない。
空表の使用高0・leafなしの完了だけを持つ後続ページは対象を変えない。均等化後の使用高が
0でも、元Eが正のreceiptを保持するページは安定検査の対象に保つ。

ordinary sourceでは元消費高・before／after・marker・hard keepを保つ最大prefixで、
容量不足を証明できる候補を先に拒否する。その他の候補は既存のranked選択を元終端まで
探索する。混在表の容量証拠は実際に選択した元cursorと実header catalog・幅・名前から
次のsource atomを求める。nearest eventとその直前の実replay、guardによるskipの件数、
current／lower／next cursorとsource atom、caption／cell／入れ子の元位置をcanonicalへ
保持する。中間guardの全preimageを保持したとは扱わない。未証明の容量区間は限界値内の
整数容量で実評価する。表の最小admission capacityと実使用高を区別し、表終端のkeepは
後続ordinary／表まで含める。実容量0で進む空表はtyped observationで保持する。

失敗候補の正のdeficitを全段から保持し、(deficit, column, source start)の順で最小値を
選んで高さを増やす。任意のfull heightへのjumpを行わない。各候補は全段の元before／after
prefix、使用高、直前spacing、実partsとblocked状態を保持する。rejection一覧だけでなく
このcapacity traceもreceiptへ束縛する。候補上限はinclusiveで、開始前に検査する。
records、work、spool、試行と再shapingの受理済みprefixを失敗・再試行・callback後にも保つ。

BookV2ColumnBalanceReceiptは同じsearch、limits、元sourceと測定、選択したページ、
baseline入力と候補列へ束縛する。不変receiptと実列高を安定選択で検査し、source closureと
数式terminalへ接続する。terminalには均等化がある場合だけreceipt fingerprintと選択高を
追加する。balance指定なしの経路は既存のcanonical bytesと課金位置を維持する。

## 検証

ordinaryの奇偶個数、空段、hard keepとwidow／orphan、元spacing／marker、named／parity、
強制改ページ、本文と脚注の共同終端、実表のcaption／span／入れ子／headerと数式を検査する。
実高0の空表と正のspacing、候補・work・record・spoolのexact／1不足、別owner、再試行と
callbackの追加消費を検査する。単段の選択認可へ変換できないことをcompile-failで確認する。

独立decoderは実probeの元source metricsとcanonical入力からE、最初の高さ、ordinaryの
hard groupと全段prefix、保持した全N段traceのblocked列とrejectionの一対一対応、発行した
実prefix証拠の最小deficitと高さ列、実配置の一回消費を照合する。全混在kernel branchの
再探索や大域的な最適均等化の証明とは扱わない。
表のcanonical certificateと元cell／caption／入れ子位置、実prefixとの対応も検査する。
これは表選択kernelと全nearest guardの独立再実装ではなく、その受入を主張しない。
既存PDFの独立検査とbyte比較を維持する。原Haranoを指定できない試験は明示gateとする。

## 残る範囲

段組display／構造／navigation／PDFと独立受入、Pageラベルの外側の収束、全commandの
physical byte／capacity／spoolと同時保持graph、公開Book /2と4 manifests、元全巻・
5,000 distinct画像・原Harano全巻・管理host・制御性能・著者／人手受入を継続する。
元本文内数式の未作成Speech／SemanticRefをこの処理から推測して補わない。
