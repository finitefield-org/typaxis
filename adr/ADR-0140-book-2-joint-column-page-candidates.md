# ADR-0140: Book /2 の全段と脚注を同じ物理ページ候補へ結ぶ

- 状態: 実装（候補適合判定。自動選択・安定配置・PDF は継続）
- 日付: 2026-10-07
- 関連: [設計 §14.277](../docs/28-vmb-book-production-compatibility.md#14277-全段と脚注を同じ物理ページ候補へ結ぶ実装追補)、[ADR-0139](ADR-0139-book-2-column-line-convergence.md)

## 問題

段幅での測定を既存の単段ページ選択へ接続すると、段ごとに物理ページが進み、
脚注を各段で独立に確定してしまう。一つの物理ページの全段から要求される脚注を
共通の領域へ収め、その最初の断片を参照と同じページへ置く必要がある。

## 決定

段組用の source flow と表の測定を別の型で保持する。元本文・定義・table ordinal、
caption／セル／入れ子・番号・名前の namespace と予算を既存の collector と共通にする。
単段 flow・測定・選択結果へ変換する API は提供しない。

共通の本文／表 kernel で source の選択と脚注の最終 fit を分ける。単段は従来どおり
直後に fit する。段組は一つの demand branch を左から右の全段へ渡し、各段の元範囲と
表の continuation を消費してから、全ページの脚注矩形へ一度だけ fit する。
新規定義の最初の実断片と依存関係の閉包を全参照に対して必要とし、収まらなければ
ページ候補全体を拒否する。継続 demand は次の物理ページに渡す。

候補は元マスターの全段に対応し、未使用の段は末尾だけに許す。物理 page index は
全段の後に一度だけ進める。本文がない継続ページでも、脚注の実 source が進む必要が
あり、空の段だけでページを増やさない。既存の keep と表の実 cursor を使用し、
普通の範囲へ表のセルを平坦化しない。名前が合わない元本文は owner 付きで拒否する。

脚注に横方向で重なる段の最大使用高を予約へ渡す。全幅の脚注では全段の最大高を使い、
一部の段だけに重なる脚注では他の段の高さを差し引かない。区切り線と marker の
実占有量、必須断片と元番号は既存の kernel から引き継ぐ。

実ページの段 slot は record へ予約してから確保する。constructor は失敗した prefix
の record／work を返し、候補失敗後も消費済み work と予約を戻さない。検索中の frame、
名前、診断状態は終了時に復元し、別候補へ状態を混ぜない。

## 検証と継続範囲

二段の参照に対する一つの脚注領域、必須定義が全て開始できない候補の拒否、脚注だけの
継続、片側／全幅の重なり、元名前、入れ子表・caption・rowspan・同一ページ内の表継続、
別 search の拒否、work の exact／1不足と失敗履歴を検査する。元 Harano でも同じ
source と cursor を検査し、三つの型の変換拒否を compile-fail で確認する。

この API は caller が提示する source cut の候補を検査する。自動候補列挙と ranking、
明示改ページを含む安定ページ列、実段幅への再組版 feedback と反復 header の variant、
last-page balance、物理配置・terminal・PDF と独立検証は引き続き接続する。
段組の候補を単段の paint receipt へ渡さず、private driver／PDF assembly の段組拒否を
維持する。検証結果は[実装台帳 §277](../docs/28-vmb-book-production-progress.md#book-2-joint-column-candidates-design-14277)へ記録する。
