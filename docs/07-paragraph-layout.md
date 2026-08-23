# Paragraph layout

ParagraphをBox、Glue、Penalty、Discretionary、InlineObjectへ正規化する。全itemがTextSpanまたはgenerated provenanceを保持する。

GreedyとOptimalは同一`ParagraphInput`/`ParagraphBreak`を使う。Optimalはbadness、penalty、fitness class、連続hyphen、overfull、最終行を評価し、同cost時のtie-breakを候補byte offset、前node順で固定する。

unit conversionとbadness計算はchecked integer/rationalで行う。final reshape後の幅変化で再breakする場合も上限とfingerprintを持つ。
