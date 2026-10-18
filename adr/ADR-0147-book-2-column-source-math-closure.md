# ADR-0147: 段組の元sourceと数式terminalを同じ実配置へ結ぶ

状態: Accepted（private Book /2 staging）
日付: 2026-10-15
対象: [VMB設計 §14.284](../docs/28-vmb-book-production-compatibility.md#book-2-column-source-math-closure-design-14284)
実装記録: [§284](../docs/28-vmb-book-production-progress.md#book-2-column-source-math-closure-design-14284)

## 問題

§14.283の安定配置は、元本文と参照された脚注定義の一回消費や、元数式receiptと実際の
段座標の対応を証明しない。反復headerでは実幅に応じて元行とは異なる行分割を使うため、
行番号の一致だけでは元sourceの所有関係を検査できない。単段のclosureやterminalへ変換して
後続の描画認可を得ることもできない。

## 決定

BookV2ColumnSourceClosureは同じsearchのBookV2ColumnStablePagesと、その正確なsequenceを
借用するBookV2ColumnPlacedSequenceを保持する。単段と共通のprivate source検査kernelへ
各実段と一つの元脚注領域を渡す。元semantic leafと非描画の改ページを一度だけ消費し、
参照された定義の終端と未参照定義の未配置を検査する。反復headerの実graph・元unit区間・
caption／セル／入れ子との対応を保持し、反復paintは元semanticの再消費に数えない。

段組のfragmentは所属する実段または元脚注領域と、元測定graphの起点から得た横差分で
検査する。public accessorは実fragmentに対応する行graphを借用し、単段のflow／測定／
closureへの変換を提供しない。with_budgeted_book_v2_column_sourcesは幅と物理配置の安定後に
closureを構築し、成功後だけcallbackへ渡す。確保前のrecord予約、走査work、失敗と再試行の
受理済みprefix、callbackの追加消費を同じcaller台帳へ戻す。

BookV2ColumnMathTerminalsは専用のclosureと実配置を借用する。inline／blockのvectorと
native数式の元receiptを、実page／fragment／段または脚注／セル／反復分類と物理座標へ
結ぶ。式番号の全ページfragment indexと実header graphを保持する。単段と共通のprivate
collector／byte encoderを用い、単段terminalの公開型とcanonical bytesを維持する。
段組のcanonical表現には元column planと実段のbounds・空rangeも束縛する。

terminal構築前に選択した元unitとblockの実幅を再検査する。元native計算とcallerのspoolを
下限として、terminalのrecords／canonical容量／走査とhash workを予約・累積する。
途中失敗でも受理済み予約とworkを返金しない。別search／stable／limitsや同内容の別graphを
受理しない。

## 検証

元本文・改ページ、全幅脚注と未参照定義、空anchor、入れ子・rowspan・captionと実幅の
反復headerを検査する。vector／native式は元receiptへの参照と物理座標、式番号のindex、
semantic／反復数を独立して照合する。foreign sequence、別stable結果のgeometry、異なる
limits、work／record／spoolの上限ちょうどと1不足、失敗後の再試行とcallbackの追加消費を
検査する。単段型への変換をcompile-failで拒否し、共通kernelを使う既存単段経路を回帰する。
原Harano試験は元ファイルを明示指定できた場合だけ実行済みと記録する。

## 残る範囲

このclosureとterminalは段組のdisplay／PDF／PDF/UA認可ではない。last_page balance、
段組の構造・navigation・PDFと独立受入、Pageラベルの外側の収束、全commandのphysical
byte／capacity／spoolと同時保持graph、公開Book /2と全manifest、元全巻・5,000 distinct画像・
原Harano全巻・管理host・制御性能・著者／人手受入を継続する。元本文内数式の未作成
Speech／SemanticRefをこの処理から推測して補わない。
