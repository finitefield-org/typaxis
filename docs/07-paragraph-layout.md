# Paragraph layout

ParagraphをimmutableなBox、Glue、Penalty、Discretionary、InlineObjectへ正規化する。Boxとtext-bearing itemはadmitted fontでshape済みのrun sliceと`BidiLevel`を保持し、全itemがparsed `TextSpan`またはdocs/05で定義した`GeneratedProvenance { key: GeneratedBufferKey, generated_text_span }`を保持する。generated identityはallocation IDを除く`(key, start_byte, end_byte)`であり、同じ`LayoutEpoch`内で一意でなければならない。Discretionaryはno-break、pre-break、post-breakの各branchについて描画content、advance、provenanceを明示し、glyph位置からbranch contentを復元しない。

GreedyとOptimalは同一`ParagraphInput`/`ParagraphBreak`を使う。Optimalはbadness、penalty、fitness class、連続hyphen、overfull、最終行を評価し、同cost時のtie-breakを候補byte offset、前node順で固定する。

unit conversionとbadness計算はchecked integer/rationalで行う。初回のshaping/break selectionはreshape passに数えず、その後の`final reshape -> width比較 -> 必要ならrebreak` feedback cycleを1 passと数える。`ResourceLimits.max_line_reshape_passes`回目まで実行でき、各入力と出力のcanonical fingerprintを記録する。最後に許可されたpassの出力もstableでなければ、そのpass完了後にL5xxx errorを返し、max+1 passを開始せず、部分的な`ParagraphBreak`を成功値として流さない。
