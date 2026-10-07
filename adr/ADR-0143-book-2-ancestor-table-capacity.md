# ADR-0143: 親表の予約後の実容量で子表を継続する

状態: Accepted（private Book /2 staging）
日付: 2026-10-07
対象: [VMB設計 §14.280](../docs/28-vmb-book-production-compatibility.md#book-2-ancestor-table-capacity-design-14280)

## 問題

Book /2の子表は、到達する空ページの容量から共通の縦位置で分割できるかを判断していた。
親の元ヘッダーや実幅で選ぶ反復ヘッダー、同じセルの先行内容を予約すると、その共通位置まで
進めない場合がある。別の親セルが終了した後も子表を開始できず、分割可能な表をOversizeにする。
§14.279の512 pt入力がこの状態を再現する。384 ptで事前に独立セル継続を選ぶ試験だけでは
この問題を検査できない。

## 決定

子表の評価へ、親が与えた実容量であることをprivateな一時contextとして渡す。元の共通分割で
進める場合はその選択を保持する。進めない場合は、表全体のkeepを守ったうえで元セルごとの
継続を試す。元caption、header、keep、改ページ、rowspanの評価規則を共用する。

開始済みの共通カーソルも扱う。元の合法なoffsetを各元セルの内容境界へ照合し、完了した
prefixを再消費しない。未消費の内容やkeepの途中にある境界はReceiptMismatchとする。
完了したセルの行paddingを許し、rowspanでは現在行の測定済みの残り高を保持する。
新しい独立セル状態をカーソルへ束縛し、継続時にはその状態を使う。

分割方針と親予約contextは試行の終了時に戻す。保存済みの共通カーソルからも再試行でき、
異なる候補を評価したことによって表全体の方針を変更しない。候補を縮める処理は、独立セル
状態を持つ実選択のcaption・cell・headerの占有量を使い、共通offsetへ読み替えない。

追加の元セル走査と境界探索は既存のworkへ計上する。元セル位置の配列と継続状態は既存の
record予約とspool上限で構築する。子searchの共有台帳を成功・失敗ともに戻し、破棄した候補や
失敗までの受理済みprefixを返金しない。凍結profileは独立セル継続を有効にしない。

## 検証

本文の480／512／544 pt、脚注の512／544 pt、captionとrowspanを持つ子表、変更していない
元Harano日本語を検査する。元semantic leafの一回消費、選択headerの実幅・実高・owner、
全配置leafのfragment内占有を確認する。512 ptのsearchについてwork／recordのexact／1不足と
失敗prefix、同じ入力での選択fingerprint一致を検査する。

共通kernelへの変更なので、全Book /2 CLI tests、旧syntaxとBook /2 syntax、layout、pagination、
workspace全featuresのcompile、旧precomposed経路を回帰対象とする。元入力・フォント・全source・
実binaryとcommand／logを固定し、既存PDFの独立検査と直前の全PDF回帰との対応を確認する。
849 PDFのbyte一致を維持し、変更した7元入力の14 PDFは元sourceと原font metricsから
実paragraph位置・ParentTree・ActualText・CID paintを照合する。ページ数とdriverのframes／
脚注領域を維持し、変更を認める入力と成果物を比較で明示する。
実行結果は[進捗台帳 §280](../docs/28-vmb-book-production-progress.md#book-2-ancestor-table-capacity-design-14280)へ記録する。

## 残る範囲

この判断は元sourceの分割選択である。選択した実段幅への本文feedback、幅が収束した段組
ページ列、last_page balance、安定配置・terminal・段組PDFの接続と独立受入を継続する。
公開Book /2、完全なcommand予算、元全巻、管理host、制御された性能と著者／人手受入を
この局所修正で完了扱いにしない。Speech／SemanticRefの未作成も継続して記録する。
