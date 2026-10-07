# ADR-0145: 元sourceと累積予算を保って段幅・ページ選択を自動収束する

状態: Accepted（private Book /2 staging）
日付: 2026-10-07
対象: [VMB設計 §14.282](../docs/28-vmb-book-production-compatibility.md#book-2-column-page-convergence-design-14282)
実装記録: [§282](../docs/28-vmb-book-production-progress.md#book-2-column-page-convergence-design-14282)

## 問題

§14.281の実段幅feedbackは短い行graphのcallbackへ属している。次の再組版へ渡すために
幅配列を未課金でcopyすると、元planへの束縛とcommandの累積予算を失う。毎反復へ
独立した予算を渡す部品試験だけでは、自動処理の失敗・再試行の上限を証明できない。
本文と反復headerを同時保持し、選択された全段・元脚注領域を再検査する必要がある。

## 決定

capture_source_width_feedbackは、同じ元flowへの参照、同じ元column plan、search ownerと
測定fingerprintを検査して、予約済みの幅・起点・元unit終端・block配列を所有権ごと移す。
新しい行graphや同内容の別flow／planへ付け替えず、過去のgraphをcallback再帰で保持しない。
新しい配列を確保せず、検査のworkと失敗prefixを元searchへ残す。

段組専用のheader catalog driverは実継続から必要な幅を発見する。元header source scopeと
予算を単段の既存処理と共有し、seed・同時再構築graph・測定・variant・catalogの型は
段組専用に保つ。発見済みのtable owner・親幅のmetadataを同じ元flowの幅反復で保持する。
毎回のgraphは新しい元seedへ束縛して再構築し、放棄した探索・配列・graphを返金しない。

with_budgeted_book_v2_column_pagesは、元resource・native計算・sourceとcolumn planを保持して
再shaping、実header幅の探索、全段と一つの脚注領域のページ選択、元unitの幅feedbackを
反復する。callerのrecord・work・開始済みline／page passを全反復と失敗・再試行へ接続する。
幅循環は一つのdigestで検出し、その後は元selected lineのend_unitを次回の指定にも渡す。
古い行番号や新しいUTF-8 offsetで置き換えない。

共通のpage安定比較kernelで、同じ不変測定から実ページ列を二回以上選択して比較する。
実段のboundsと使用高、item range・表のfingerprintとcursor、改ページ・名前・demand、
脚注の断片・候補履歴を照合する。BookV2ColumnRepeatedPagesはこのsource選択の証明であり、
balance・物理配置・PDFのreceiptや単段の安定配置へ変換しない。

本文とblockの実幅が一致し、前回の元unit幅・起点も一致した後だけcallbackを呼ぶ。
予算不足や不一致でcallbackを呼ばず、途中失敗の受理済みprefixをcallerへ戻す。
callback自身が同じsearchで追加消費した場合も、呼出し後のprefixを保持する。

## 検証

予約済み配列のaddressがcallback内外で一致し、同内容の別flow／planと別searchを拒否する。
元文字・glyph owner、全幅脚注・未参照定義・空anchor、本文／脚注の入れ子・rowspan・caption・
反復header、vector／native式とPNG／JPEG／SVGの20ケースを検査する。元Haranoを無変更で使い、
1段と1／2段混在の実幅循環で元行終端を保持する経路も検査する。

累積work／record／line／page passのexact／1不足、失敗後の再試行と別limitsの拒否を確認する。
大きい入れ子表は明示的な十分なwork上限で収束を確認し、低い上限での拒否も維持する。
固定したsource・font・実binary・commandsと回帰結果を実装台帳 §282へ対応付ける。

## 残る範囲

今回の自動処理は元source幅と繰り返したページ選択の収束である。last_page balance、
実段の物理配置、source closure・math terminals・段組PDFと独立検査を接続する必要がある。
Page参照の最終ラベルとPDFまでの外側の収束、全commandのphysical byte／capacity／spool・
同時保持graphの監査、公開Book /2と全manifest、元全巻・5,000 distinct画像・管理host・
制御性能・著者／人手受入も継続する。未作成のSpeech／SemanticRefを補った成果物や
設計全体の完了とは扱わない。
