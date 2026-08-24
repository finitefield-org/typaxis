# Paragraph layout

ParagraphをimmutableなBox、Glue、Penalty、Discretionary、InlineObjectへ正規化する。Boxとtext-bearing itemはadmitted fontでshape済みのrun sliceと`BidiLevel`を保持し、全itemがparsed `TextSpan`またはdocs/05で定義した`GeneratedProvenance { key: GeneratedBufferKey, generated_text_span }`を保持する。generated identityはallocation IDを除く`(key, start_byte, end_byte)`であり、同じ`LayoutEpoch`内で一意でなければならない。Discretionaryはno-break、pre-break、post-breakの各branchについて描画content、advance、provenanceを明示し、glyph位置からbranch contentを復元しない。

ASTのSoftBreak/HardBreakは空のpackage-registered `discretionary` generated siteへbindしたzero-width Penaltyとしてlogical位置に明示する。SoftBreakはallowed、HardBreakはmandatoryであり、paragraph末尾のSoftBreakはterminal mandatoryへ昇格する。explicit breakが最終text clusterの後にある場合、同じlogical位置のUnicode terminal penaltyはprohibitedへ置換し、break node側だけがterminal legalityを所有する。

`CanonicalParagraph`はcanonical itemizerが発行したexact `ItemizedShapeRequests`、そのrequestsから得たvalidated run列、package text receiptを同時に照合し、paragraph levelも同じitemizer ownerから保持する。empty generated siteだけはitemizer workを発行せず、empty text/empty run列をpackage receiptへ再照合する。broken lineのUAX #9処理はL1後のlogical level列とL2 `visual_to_logical` cluster permutationを別々に保持し、glyph orderをsource orderとして再利用しない。

reference paint pipelineもこの順序を型で保持する。`LineLevelsAfterL1`を発行後、selected lineの全clusterをexact validated runへ再bindするfinal-line stageを通し、nonterminal lineのGlueをpriority昇順・logical orderのchecked proportional roundingでtarget widthまでjustificationしてからだけL2 permutationを発行する。terminal lineも明示no-adjust policyとしてjustification stageを通る。L1 reset後のlevelを最終Display glyph runへ保持する。

default line-break ownerはUnicode 16.0 / UAX #14 revision 53の生成済みproperty tableを使い、paragraph内の全package text siteをlogical順に連結してboundaryを決める。Unicode boundaryはvalidated shaping-cluster endとの共通部分だけをParagraph itemへ昇格し、cluster内部のallowed boundaryは抑止し、mandatory boundaryがcluster内部に現れたrunは拒否する。U+0020も常に暗黙breakとはせず、Glue直後のexplicit Penaltyが`SP × WJ`を含むUnicode legalityを保持する。生成tableはhash固定したUnicode Character Databaseから再生成でき、unmodified `LineBreakTest-16.0.0.txt`の全16,672 caseをverification gateにする。complex-context `SA` letterはdefault `AL` resolutionであり、辞書分割と日本語tailoringはresolved data-table layerで別途適用する。

GreedyとOptimalは同一`ParagraphInput`/`ParagraphBreak`を使う。Optimalはbadness、penalty、fitness class、連続hyphen、overfull、最終行を評価し、同cost時のtie-breakを候補byte offset、前node順で固定する。

unit conversionとbadness計算はchecked integer/rationalで行う。初回のshaping/break selectionはreshape passに数えず、その後の`final reshape -> width比較 -> 必要ならrebreak` feedback cycleを1 passと数える。`LineReshapeFeedback`はdomain-separated `LineLayoutStateFingerprint`を各passのinput/outputについて記録し、one-shot `LineReshapePassPermit`の発行前にbudgetをconsumeする。permitを完了せず破棄したownerはfail closedになり、同じpassを再発行しない。`ResourceLimits.max_line_reshape_passes`回目まで実行でき、最後に許可されたpassの出力もstableでなければ、そのpassのoutput fingerprintを記録した後にL5xxx errorを返し、max+1 passを開始せず、部分的な`ParagraphBreak`を成功値として流さない。
