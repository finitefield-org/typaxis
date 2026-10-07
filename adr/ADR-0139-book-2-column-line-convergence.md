# ADR-0139: Book /2 の段幅での本文再組版

- 状態: 実装（行の測定・収束。物理ページの選択・描画は継続）
- 日付: 2026-10-07
- 関連: [設計 §14.276](../docs/28-vmb-book-production-compatibility.md#14276-段幅で元本文を再組版する実装追補)、[ADR-0138](ADR-0138-book-2-column-frame-plan.md)

## 問題

元マスターの段幅を求めても、既存の本文再組版は単段のページ計画だけを受け取る。
段の測定結果をそのまま既存のページ選択へ渡すと、段を物理ページとして扱うことになる。
実際の字形・行境界・表と脚注の幅を測定し、物理ページの選択とは区別する必要がある。

## 決定

`with_budgeted_book_v2_column_lines` は元 `BookV2ColumnFramePlan` の測定矩形を使う。
独自の字形生成・行分割を追加せず、既存の実 shaping、inline の準備、行の選択と
line-context を反映した再 shaping の収束処理を共有する。元リソース、native math、
caller の候補探索・開始済み再組版 pass・source record の履歴をそのまま引き継ぐ。
収束を実際に比較できた場合だけ callback を呼ぶ。別 source の計画は開始前に拒否する。

本文には到達する最大実段幅の測定矩形を使い、表の固定／割合列、caption、セル、
入れ子と段落の indent を既存の投影で測定する。脚注は元の全ページ脚注領域を使い、
実際の番号マーカーと gap の占有幅を差し引く。元 source に結び付く幅の候補も扱う。
これらの測定幅は、選択済みの物理ページや段の配置 receipt ではない。

結果は `BookV2ConvergedColumnLines` として公開し、単段の収束結果への変換を設けない。
行の frame は単段計画と段組計画を区別して保持する。既存の本文ページ選択は段組 frame
を `PendingRegion("column_pages")` で拒否し、counted constructor はそれまでの record
履歴を返す。private PDF driver と PDF assembly の段組拒否も引き続き維持する。

## 検証と継続範囲

実字体からの折り返し、元文字と glyph owner、幅候補、別 source、候補 work の exact／
1不足、pass 不足、source record 不足、失敗と再試行の台帳、未収束時の callback 拒否を
検査する。固定列・割合列・caption・header・colspan・rowspan を含む入れ子表を、本文と
脚注の双方で測定する。元 Harano の SHA-256 を確認して同じ検査を行う。型の変換拒否は
compile-fail で確認する。コマンドと結果は[実装台帳 §276](../docs/28-vmb-book-production-progress.md#book-2-column-line-convergence-design-14276)へ記録する。

同じ物理ページの全段と脚注を共通に選択する処理、元 cursor の継続、改ページ・keep、
表の反復 header、最後の物理ページの balance、実段 PDF の描画と独立検査は残る。
この変更で段組 PDF や設計全体の受入を完了したとは扱わない。
