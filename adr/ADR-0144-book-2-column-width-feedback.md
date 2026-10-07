# ADR-0144: 選択した実段幅を元論理単位の再組版へ戻す

状態: Accepted（private Book /2 staging）
日付: 2026-10-07
対象: [VMB設計 §14.281](../docs/28-vmb-book-production-compatibility.md#book-2-column-width-feedback-design-14281)

## 問題

段組みの初期行は、到達する最大段幅で計測する。自動選択した物理ページの実段幅や
脚注全幅が異なる場合、元行をそのまま配置できない。再組版で行数が変わるため、
現在の行番号を次の幅指定へ使うと元の文字・数式・参照の対応が失われる。表内では
元のcaption、セル、入れ子と反復headerを別々に照合する必要がある。

## 決定

BookV2ColumnPageSearchに専用のparagraph_frame_feedbackを追加する。元search owner・
column planへの同一参照・測定fingerprint・完全な元ページ列を検査してから、
BookV2ColumnWidthFeedbackを構築する。元flowのfingerprintと元paragraph ownerを保持する。

本文の各実段と、ページ全体の元脚注領域を別に扱う。実選択の行を元unit区間へ戻し、
元の字下げと脚注番号・gapを含む幅差を適用する。一つの元unitを重複消費せず、
観測した段落は全unitが一度だけsemanticに対応することを確認する。空段落の
0-unit行を明示的に扱い、未参照の脚注定義は元の測定幅を保持する。

表の元root親幅を実段／脚注の幅へ変え、既存の階層再投影を共用する。caption・
セル・入れ子の元leafごとに幅とstartを返す。別graphで選ぶ反復headerは実frameと
測定frameを検査し、元semantic幅を上書きしない。rootの実出現を要求し、
未参照の定義を出現済みとして扱わない。図版・native／vector式は元ownerの
block幅・startとして同じ再組版へ戻す。

幅循環のconsumerは、元selected lineのend_unitを保持して次のsource-width指定へ渡せる。
同じsearch・plan・測定・flowへの所属を検査し、すでに保持した候補の再保持を拒否する。
行境界を含む候補fingerprintを更新する。このAPI自身は循環検出やcommand収束の
実行主体ではなく、callerが必要な履歴と累積予算を保持する。

元unitの幅・訪問・start、block・表出現・元行終端の配列は確保前にlogical recordへ
予約する。走査と候補hashを既存workへ計上し、失敗までの受理済みprefixをsearchに
保持する。結果は単段feedbackや安定配置の型へ変換しない。

## 検証

元unit幅を新しいshapingへ渡して段ページを再選択する。本文140／220 pt、
脚注300／380 ptから元番号・gapを予約した幅、未参照定義、空anchor、
入れ子・rowspan・captionと反復headerのowner、原Harano日本語を検査する。
vector／native式とPNG／JPEG／SVGの本文・脚注・入れ子表20ケースを扱う。
同じ入力の再選択の一致、保持行境界、異なるsearchの拒否、work／recordの
exact／1不足と失敗prefixを確認する。

全Book /2 CLI・共有layout／pagination・workspace型検査と旧経路を回帰し、
既存PDFの独立検査と前段の全成果物のbyte一致を確認する。command・実binary・
元source／font・検査器をhashで対応付ける。実行結果は
[進捗台帳 §281](../docs/28-vmb-book-production-progress.md#book-2-column-width-feedback-design-14281)へ記録する。

## 残る範囲

今回の反復試験は部品の幅対応を確認する。全commandの累積予算を持つ自動column driver、
安定した実段ページ列、last_page balance、配置・source closure・terminal・段組PDFを
接続する必要がある。callbackの短い行graph参照に属するfeedbackを外側へ渡す際には、
元source／planのownerと予約済み所有データを保つcapture経路を設ける。未課金の配列copyや
過去の全行graphを保持するcallback再帰で回避しない。

公開Book /2と全manifest、完全なcommand予算、元全巻・5,000画像・管理host・制御性能・
著者／人手受入も継続する。未作成のSpeech／SemanticRefを補った成果物と扱わず、
設計全体の完了を主張しない。
